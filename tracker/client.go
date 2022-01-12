package tracker

import (
	"context"
	"fmt"
	"log"
	"time"

	"google.golang.org/api/sheets/v4"
)

// TrackedData is a type wrap for data that is being tracked. It'll be written as string anyway.
type TrackedData []string

// GetDataFn is w function that returns data that will be written to google sheet cell.
type GetDataFn func(ctx context.Context) (TrackedData, error)

// Direction is a wrapper for direction.
// If Direction is a character e.g. 'A', 'B', 'C' data will be written in given column row by row.
// If Direction is a number e.g. '1', '2', '3' data will be written in given row column by column.
type Direction string

// TrackingTask holds information about tracking task.
type TrackingTask struct {
	fn            GetDataFn
	direction     Direction
	withTimestamp bool // if set, timestmap will be written next to written data.
	// timestampBefore indicates place of timestmap.
	// If false, timestamp will be written before data(row or column before).
	// If true, timestamp will be written after data(row or column after).
	timestampAfter bool
	interval       time.Duration // how often task will be run.
}

// trackingOption is a function that sets TrackingTask fields.
type trackingOption func(*TrackingTask)

// WithTimestamp sets withTimestamp.
// If after is true, timestamp will be written after(row or column) the data.
func WithTimestamp(after bool) trackingOption {
	return func(tt *TrackingTask) {
		tt.withTimestamp = true
		tt.timestampAfter = after
	}
}

// wrappedGetDataFn is a wrapper for GetDataFn that gets data and writes it.
type wrappedGetDataFn func(ctx context.Context) error

// TODO: set in Tracker.
const (
	spreadsheetID = "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg"
)

// Tracker is a wrapper for the Google Sheets API.
// It is used to track various kind of things and keep that data in a Google Sheet.
type Tracker struct {
	srv   *sheets.Service
	log   *log.Logger
	tasks []TrackingTask
}

// NewTracker creates new instance of Tracker.
func NewTracker(svc *sheets.Service, log *log.Logger) *Tracker {
	return &Tracker{
		srv: svc,
		log: log,
	}
}

// AddTrackingFn adds TrackingFn to set of saved TrackingFns.
func (t *Tracker) AddTrackingFn(direction Direction, interval time.Duration, fn GetDataFn, opts ...trackingOption) {
	tt := &TrackingTask{
		fn:        fn,
		direction: direction,
		interval:  interval,
	}
	for _, opt := range opts {
		opt(tt)
	}
	t.tasks = append(t.tasks, *tt)
}

// wrapWithSheetsService wraps TrackinTask data into single function that finds place to write
// data from TrackingTask and writes it.
func (t *Tracker) wrapWithSheetsService(task TrackingTask) wrappedGetDataFn {
	return func(ctx context.Context) error {
		data, err := task.fn(ctx)
		if err != nil {
			return err
		}

		resp, err := t.srv.Spreadsheets.Values.Get(spreadsheetID, fmt.Sprintf("%s:%s", task.direction, task.direction)).Do()
		if err != nil {
			return err
		}

		// TODO: support column write for now.
		elementLen := len(resp.Values)
		// dataLen := len(data)

		t1 := time.Now().String()

		var vr sheets.ValueRange
		for _, dataPoint := range data {
			values := []interface{}{dataPoint}
			if task.withTimestamp {
				values = append(values, t1)
			}
			vr.Values = append(vr.Values, values)
		}

		_, err = t.srv.Spreadsheets.Values.
			Update(spreadsheetID, fmt.Sprintf("%s%d", task.direction, elementLen+1), &vr).
			ValueInputOption("RAW").
			Context(ctx).
			Do()

		return err
	}
}

// Start stars running all tasks.
func (t *Tracker) Start(ctx context.Context) {
	for _, task := range t.tasks {
		go runTask(ctx, task.interval, t.wrapWithSheetsService(task))
	}
}

// runTask runs given wrappedGetDataFn function till context is done with given interval.
func runTask(ctx context.Context, interval time.Duration, fn wrappedGetDataFn) {
	ticker := time.NewTicker(interval)
	for {
		select {
		case <-ticker.C:
			log.Println("runTask: tick")
			if err := fn(ctx); err != nil {
				log.Println(err)
			}

		case <-ctx.Done():
			log.Println("runTask: context closed")
			return
		}
	}

}
