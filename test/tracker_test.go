package integration_test

import (
	"context"
	"log"
	"testing"
	"time"

	"data-tracker/api"
	"data-tracker/tracker"

	"github.com/stretchr/testify/require"
)

// TestTracking tests if tracking works.
func TestTracking(t *testing.T) {
	wrapper, err := api.NewAPIWrapperWithInit(context.Background())
	require.NoError(t, err)

	tr := tracker.NewTracker(wrapper.Service(), log.Default())

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*15)
	defer cancel()

	data := []string{randomData(), randomData(), randomData()}
	done := make(chan struct{}) // will be closed after data is written.

	tt := tracker.NewTrackingTask(
		spreadsheetID,
		tracker.Direction("A"),
		time.Second,
		func(ctx context.Context) (tracker.TrackedData, error) {
			return tracker.TrackedData(data), nil
		},
		tracker.WithCallback(func(err error) { close(done) }),
	)

	tr.AddTrackingFn(tt)
	tr.Start(ctx)
	<-done
	column, err := wrapper.GetColumn(spreadsheetID, "A1:A3")
	require.NoError(t, err)
	require.Equal(t, stringSliceToInterfaceSlice(data), column)
}
