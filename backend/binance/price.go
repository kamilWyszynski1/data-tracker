package binance

import "net/url"

type symbolFn func() string

func newSymbolFn(s string) symbolFn {
	return func() string {
		return s
	}
}

func (s symbolFn) EmbedData(q *url.Values) {
	q.Set("symbol", s())

}

type CurrentAveragePriceResponse struct {
	Mins  int    `json:"mins"`
	Price string `json:"price"`
}
