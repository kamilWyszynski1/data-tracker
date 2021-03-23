package berror

import "errors"

var (
	ErrInvalidData = errors.New("invalid data")
)

type BinanceCliErr struct {
	Err error
	Msg string
}

func (b BinanceCliErr) Error() string {
	return b.Err.Error()
}
