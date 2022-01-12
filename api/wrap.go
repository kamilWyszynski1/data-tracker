package api

import (
	"context"
	"data-tracker/env"
	"fmt"
	"io/ioutil"
	"os"

	"golang.org/x/oauth2/google"
	"google.golang.org/api/option"
	"google.golang.org/api/sheets/v4"
)

type APIWrapper struct {
	srv *sheets.Service
}

// NewAPIWrapperWithInit returns new APIWrapper instance.
// Function initializes google sheets api.
func NewAPIWrapperWithInit(ctx context.Context) (*APIWrapper, error) {
	b, err := ioutil.ReadFile(os.Getenv(env.CREDENTIALS_FILE_PATH))
	if err != nil {
		return nil, fmt.Errorf("unable to read client secret file: %w", err)
	}

	// If modifying these scopes, delete your previously saved token.json.
	config, err := google.ConfigFromJSON(b, GoogleSheetsAuthURL)
	if err != nil {
		return nil, fmt.Errorf("unable to parse client secret file to config: %w", err)
	}
	client := GetClient(config)

	srv, err := sheets.NewService(ctx, option.WithHTTPClient(client))
	if err != nil {
		return nil, fmt.Errorf("unable to retrieve Sheets client: %w", err)
	}
	return &APIWrapper{srv: srv}, nil
}

func NewAPIWrapper(srv *sheets.Service) *APIWrapper {
	return &APIWrapper{srv: srv}
}

// Get wraps get method.
func (a APIWrapper) Get(spreadsheetID string, range_ string) ([][]interface{}, error) {
	resp, err := a.srv.Spreadsheets.Values.Get(spreadsheetID, range_).Do()
	if err != nil {
		return nil, err
	}
	return resp.Values, nil
}

// GetRow wraps get method but returns values from single row.
func (a APIWrapper) GetRow(spreadsheetID string, range_ string) ([]interface{}, error) {
	resp, err := a.srv.Spreadsheets.Values.Get(spreadsheetID, range_).Do()
	if err != nil {
		return nil, err
	}
	return resp.Values[0], nil
}

// GetColumn get method but returns values from single column
func (a APIWrapper) GetColumn(spreadsheetID string, range_ string) ([]interface{}, error) {
	resp, err := a.srv.Spreadsheets.Values.Get(spreadsheetID, range_).Do()
	if err != nil {
		return nil, err
	}
	vals := make([]interface{}, len(resp.Values))
	for i, r := range resp.Values {
		vals[i] = r[0]
	}
	return vals, nil
}

func (a APIWrapper) Write(ctx context.Context, spreadsheetID string, range_ string, data [][]interface{}) error {
	var vr sheets.ValueRange
	vr.Values = append(vr.Values, data...)

	_, err := a.srv.Spreadsheets.Values.
		Update(spreadsheetID, range_, &vr).
		ValueInputOption("RAW").
		Context(ctx).
		Do()

	return err
}
