package models

import (
	"net/url"
	"strconv"
	"time"
)

type AllOrdersRequest struct {
	Timestamp time.Time
}

func (a AllOrdersRequest) Validate() error {
	return nil // TODO
}

func (a AllOrdersRequest) EmbedData(q *url.Values) {
	q.Set("timestamp", strconv.Itoa(timeToMilliseconds(a.Timestamp)))
}

type AllOrdersResponse struct {
	Orders []Order
}

type Order struct {
	Orderlistid       int       `json:"orderListId"`
	Contingencytype   string    `json:"contingencyType"`
	Liststatustype    string    `json:"listStatusType"`
	Listorderstatus   string    `json:"listOrderStatus"`
	Listclientorderid string    `json:"listClientOrderId"`
	Transactiontime   int64     `json:"transactionTime"`
	Symbol            string    `json:"symbol"`
	Orders            OrderSpec `json:"orders"`
}

type OrderSpec struct {
	Symbol        string `json:"symbol"`
	Orderid       int    `json:"orderId"`
	Clientorderid string `json:"clientOrderId"`
}
