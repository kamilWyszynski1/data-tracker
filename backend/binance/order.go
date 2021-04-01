package binance

import (
	"fmt"
	"net/url"
	"strconv"
	"time"
)

type OrderRequest struct {
	Symbol        string    `json:"symbol"`
	Side          string    `json:"side"`
	Type          string    `json:"type"` // MARKET
	Quantity      float64   `json:"quantity"`
	QuoteOrderQty float64   `json:"quote_order_qty"`
	Timestamp     time.Time `json:"timestamp"`
	TimeInForce   string
	Price         float64
}

func (o OrderRequest) EmbedData(q *url.Values) {
	if !o.Timestamp.IsZero() {
		q.Set("timestamp", strconv.Itoa(timeToMilliseconds(o.Timestamp)))

	}
	q.Set("symbol", o.Symbol)
	q.Set("side", o.Side)
	q.Set("type", o.Type)
	if o.Quantity != 0 {
		q.Set("quantity", fmt.Sprintf("%.2f", o.Quantity))
	}
	if o.QuoteOrderQty != 0 {
		q.Set("quoteOrderQty", fmt.Sprintf("%.2f", o.QuoteOrderQty))
	}
	if o.Price != 0 {
		q.Set("price", fmt.Sprintf("%.2f", o.Price))
	}
	if o.TimeInForce != "" {
		q.Set("timeInForce", o.TimeInForce)
	}

}

type OrderResponse struct {
	Symbol              string `json:"symbol"`
	Orderid             int    `json:"orderId"`
	Orderlistid         int    `json:"orderListId"`
	Clientorderid       string `json:"clientOrderId"`
	Transacttime        int64  `json:"transactTime"`
	Price               string `json:"price"`
	Origqty             string `json:"origQty"`
	Executedqty         string `json:"executedQty"`
	Cummulativequoteqty string `json:"cummulativeQuoteQty"`
	Status              string `json:"status"`
	Timeinforce         string `json:"timeInForce"`
	Type                string `json:"type"`
	Side                string `json:"side"`
}
