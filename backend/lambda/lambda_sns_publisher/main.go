package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/aws/aws-lambda-go/lambda"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/sns"
	"github.com/binanceBot/backend/binance"
	"github.com/binanceBot/backend/lambda/common"
)

func main() {
	lambda.Start(handler)
}

func handler() {
	binanceApiKey := common.EnvOrFatal("BINANCE_API_KEY")
	binanceSecretKey := common.EnvOrFatal("BINANCE_SECRET_KEY")
	arn := common.EnvOrFatal("TOPIC_ARN")
	subject := fmt.Sprintf("%s:%s", common.EnvOrFatal("MONGO_DB"), common.EnvOrFatal("MONGO_COLL"))

	bCli := binance.NewBinance(http.DefaultClient, "https://api.binance.com", binanceApiKey, []byte(binanceSecretKey))

	a, err := bCli.Account(binance.AccountRequest{Timestamp: time.Now().Add(-time.Second)})
	if err != nil {
		log.Fatalf("cannot get account info, %s", err)
	}
	a.TrimEmptyBalances()

	r := common.Ratios{}
	rat := make(map[string]float64, len(a.Balances))
	for _, balance := range a.Balances {
		ratio, err := bCli.CurrentAveragePrice(fmt.Sprintf("%sBUSD", balance.Asset))
		if err != nil {
			log.Printf("failed to get current average price for %s, because: %s", balance.Asset, err)
		}
		rat[balance.Asset] = ratio
	}
	r.Ratios = rat
	b, err := json.Marshal(r)
	if err != nil {
		log.Fatal(err)
	}

	// Initialize a session that the SDK will use to load
	// credentials from the shared credentials file. (~/.aws/credentials).
	sess := session.Must(session.NewSessionWithOptions(session.Options{
		SharedConfigState: session.SharedConfigEnable,
	}))

	svc := sns.New(sess, aws.NewConfig().WithRegion("us-west-1"))

	msg := string(b)

	result, err := svc.Publish(&sns.PublishInput{
		Message:  &msg,
		TopicArn: &arn,
		Subject:  &subject,
	})
	if err != nil {
		log.Fatal(err.Error())
	}

	log.Println(*result.MessageId)
}
