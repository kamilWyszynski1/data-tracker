package binance

import (
	"net/url"
	"strconv"
	"time"
)

// #################
// #### ACCOUNT  ###
// #################

type AccountRequest struct {
	Timestamp time.Time
}

func (a AccountRequest) Validate() error {
	return nil // TODO
}

func (a AccountRequest) EmbedData(q *url.Values) {
	q.Set("timestamp", strconv.Itoa(timeToMilliseconds(a.Timestamp)))
}

type AccountResponse struct {
	Makercommission  int    `json:"makerCommission"`
	Takercommission  int    `json:"takerCommission"`
	Buyercommission  int    `json:"buyerCommission"`
	Sellercommission int    `json:"sellerCommission"`
	Cantrade         bool   `json:"canTrade"`
	Canwithdraw      bool   `json:"canWithdraw"`
	Candeposit       bool   `json:"canDeposit"`
	Updatetime       int    `json:"updateTime"`
	Accounttype      string `json:"accountType"`
	Balances         []struct {
		Asset  string `json:"asset,omitempty"`
		Free   string `json:"free"`
		Locked string `json:"locked"`
	} `json:"balances"`
	Permissions []string `json:"permissions"`
}
