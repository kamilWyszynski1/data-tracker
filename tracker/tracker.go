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

// Callback is a function that will be run after data will be written.
// Handy in debuging and testing
type Callback func(err error)

// TrackingTask holds information about tracking task.
type TrackingTask struct {
	spreadsheetID string // spreadsheet where data will be written.
	// sheet is a exact sheet of spreadsheet. Default is empty, first sheet.
	sheet         string
	fn            GetDataFn
	direction     Direction
	withTimestamp bool // if set, timestamp will be written next to written data.
	// timestampBefore indicates place of timestamp.
	// If false, timestamp will be written before data(row or column before).
	// If true, timestamp will be written after data(row or column after).
	timestampAfter bool
	interval       time.Duration // how often task will be run.
	// callbacks will be run after whole writting is done.
	callbacks []Callback
}

// taskOption is a function that sets TrackingTask fields.
type taskOption func(*TrackingTask)

// WithTimestamp sets withTimestamp.
// If after is true, timestamp will be written after(row or column) the data.
func WithTimestamp(after bool) taskOption {
	return func(tt *TrackingTask) {
		tt.withTimestamp = true
		tt.timestampAfter = after
	}
}

// WithSheet sets sheet.
func WithSheet(sheet string) taskOption {
	return func(tt *TrackingTask) {
		tt.sheet = sheet
	}
}

// WithCallback adds one callback to TrackingTask.
func WithCallback(c Callback) taskOption {
	return func(tt *TrackingTask) {
		tt.callbacks = append(tt.callbacks, c)
	}
}

// NewTrackingTask returns new instance of TrackingTask.
func NewTrackingTask(spreadshetID string, direction Direction, interval time.Duration, fn GetDataFn, opts ...taskOption) TrackingTask {
	tt := &TrackingTask{
		spreadsheetID: spreadshetID,
		fn:            fn,
		direction:     direction,
		interval:      interval,
	}
	for _, opt := range opts {
		opt(tt)
	}
	return *tt
}

// wrappedGetDataFn is a wrapper for GetDataFn that gets data and writes it.
type wrappedGetDataFn func(ctx context.Context) error

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
func (t *Tracker) AddTrackingFn(tt TrackingTask) {
	t.tasks = append(t.tasks, tt)
}

// wrapWithSheetsService wraps TrackinTask data into single function that finds place to write
// data from TrackingTask and writes it.
func (t *Tracker) wrapWithSheetsService(task TrackingTask) wrappedGetDataFn {
	runCallbacks := func(err error) {
		for _, cb := range task.callbacks {
			cb(err)
		}
	}

	return func(ctx context.Context) (err error) {
		defer runCallbacks(err)

		data, err := task.fn(ctx)
		if err != nil {
			return err
		}

		range_ := AddSheetToRange(task.sheet, fmt.Sprintf("%s:%s", task.direction, task.direction))
		resp, err := t.srv.Spreadsheets.Values.Get(task.spreadsheetID, range_).Do()
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

		range_ = AddSheetToRange(task.sheet, fmt.Sprintf("%s%d", task.direction, elementLen+1))
		_, err = t.srv.Spreadsheets.Values.
			Update(task.spreadsheetID, range_, &vr).
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

// AddSheetToRange adds sheet before range.
func AddSheetToRange(sheet, range_ string) string {
	if sheet == "" {
		return range_
	}
	return fmt.Sprintf("%s!%s", sheet, range_)
}
