package models

import (
	"net/url"
	"strconv"
	"time"

	"github.com/binanceBot/backend/binance/berror"
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

func (m MyTradesRequest) Validate() error {
	if m.Symbol == "" {
		return berror.BinanceCliErr{
			Err: berror.ErrInvalidData,
			Msg: "symbol field is mandatory",
		}
	} else if m.Timestamp.IsZero() {
		return berror.BinanceCliErr{
			Err: berror.ErrInvalidData,
			Msg: "timestamp field is mandatory",
		}
	}
	return nil
}

func (m MyTradesRequest) EmbedData(q *url.Values) {
	q.Set("symbol", m.Symbol)
	q.Set("timestamp", strconv.Itoa(timeToMilliseconds(m.Timestamp)))
}

type MyTradesResponse struct {
	Trades []Trades
}

type Trades struct {
	Symbol          string `json:"symbol"`
	ID              int    `json:"id"`
	Orderid         int    `json:"orderId"`
	Orderlistid     int    `json:"orderListId"`
	Price           string `json:"price"`
	Qty             string `json:"qty"`
	Quoteqty        string `json:"quoteQty"`
	Commission      string `json:"commission"`
	Commissionasset string `json:"commissionAsset"`
	Time            int64  `json:"time"`
	Isbuyer         bool   `json:"isBuyer"`
	Ismaker         bool   `json:"isMaker"`
	Isbestmatch     bool   `json:"isBestMatch"`
}
