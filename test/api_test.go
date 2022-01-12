package integration_test

import (
	"context"
	"data-tracker/api"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

// File contains integration tests for api package.

const (
	spreadsheetID = "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg" // test spreadsheet.
)

func TestWrapper(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), time.Minute)
	defer cancel()
	wrapper, err := api.NewAPIWrapperWithInit(ctx)

	require.NoError(t, err)

	data := [][]interface{}{
		{"A1", "B1"},
		{"A2", "B2"},
		{"A3", "B3"},
		{"A4", "B4"},
	}

	require.NoError(t, wrapper.Write(ctx, spreadsheetID, "A1:B4", data))

	got, err := wrapper.Get(spreadsheetID, "A1:B4")
	require.NoError(t, err)
	require.Equal(t, data, got)

	row, err := wrapper.GetRow(spreadsheetID, "A1:B1")
	require.NoError(t, err)
	require.Equal(t, []interface{}{"A1", "B1"}, row)

	column, err := wrapper.GetColumn(spreadsheetID, "A1:A4")
	require.NoError(t, err)
	require.Equal(t, []interface{}{"A1", "A2", "A3", "A4"}, column)
}
