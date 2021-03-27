package binance

import (
	"net/url"
	"strconv"
	"time"

	"go.mongodb.org/mongo-driver/bson"
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
	Makercommission  int        `json:"makerCommission" bson:"-"`
	Takercommission  int        `json:"takerCommission" bson:"-"`
	Buyercommission  int        `json:"buyerCommission" bson:"-"`
	Sellercommission int        `json:"sellerCommission"  bson:"-"`
	Cantrade         bool       `json:"canTrade" bson:"-"`
	Canwithdraw      bool       `json:"canWithdraw" bson:"-"`
	Candeposit       bool       `json:"canDeposit" bson:"-"`
	Updatetime       int        `json:"updateTime" bson:"-"`
	Accounttype      string     `json:"accountType" bson:"-"`
	Balances         []*Balance `json:"balances"`
	Permissions      []string   `json:"permissions" bson:"-"`
}

type Balance struct {
	Asset  string `json:"asset,omitempty" json:"bson,omitempty"`
	Free   string `json:"free" bson:"free,omitempty"`
	Locked string `json:"locked" bson:"locked,omitempty"`
}

func (a *AccountResponse) TrimEmptyBalances() {
	balances := make([]*Balance, 0)
	for _, b := range a.Balances {

		v, err := strconv.ParseFloat(b.Free, 64)
		if err != nil {
			continue
		}
		if v != 0 {
			balances = append(balances, b)
		}
	}
	a.Balances = balances
}

func (a *AccountResponse) MarshalBSON() ([]byte, error) {
	a.TrimEmptyBalances()
	return bson.Marshal(*a)
}

type AccountResponseWithTimestamp struct {
	AccountResponse `bson:",inline"`
	Timestamp       time.Time `json:"timestamp" bson:"timestmap"`
}

func (a *AccountResponseWithTimestamp) MarshalBSON() ([]byte, error) {
	a.TrimEmptyBalances()
	return bson.Marshal(*a)
}
