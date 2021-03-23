package binance

import (
	"net/url"
	"strconv"
	"time"
)

type MyTradesRequest struct {
	Symbol     string
	StartTime  time.Time
	EndTime    time.Time
	FromID     int // TradeId to fetch from. Default gets most recent trades.
	Limit      int // Default 500; max 1000.
	RecvWindow int // The value cannot be greater than 60000
	Timestamp  time.Time
}

func (m MyTradesRequest) EmbedInURL(u *url.URL) {
	u.Query().Add("symbol", m.Symbol)
	u.Query().Add("timestmap", strconv.Itoa(int(m.Timestamp.Unix())))
}

func (m MyTradesRequest) Validate() error {
	if m.Symbol == "" {
		return BinanceCliErr{
			Err: ErrInvalidData,
			Msg: "symbol field is mandatory",
		}
	} else if m.Timestamp.IsZero() {
		return BinanceCliErr{
			Err: ErrInvalidData,
			Msg: "timestamp field is mandatory",
		}
	}
	return nil
}
