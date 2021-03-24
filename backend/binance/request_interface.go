package binance

import (
	"net/url"
	"time"
)

type RequestInterface interface {
	EmbedData(*url.Values)
}

var (
	timeToMilliseconds = func(t time.Time) int {
		return int(t.UnixNano()) / 1e6
	}
)
