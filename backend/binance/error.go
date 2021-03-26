package binance

import "fmt"

type BinanceError struct {
	Code int    `json:"code"`
	Msg  string `json:"msg"`
}

func (b BinanceError) Error() string {
	return fmt.Sprintf("%s, with %d code", b.Msg, b.Code)
}
