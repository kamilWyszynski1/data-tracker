package main

import (
	"encoding/json"
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

	bCli := binance.NewBinance(http.DefaultClient, "https://api.binance.com", binanceApiKey, []byte(binanceSecretKey))

	a, err := bCli.Account(binance.AccountRequest{Timestamp: time.Now().Add(-time.Second)})
	if err != nil {
		log.Fatalf("cannot get account info, %s", err)
	}

	a.TrimEmptyBalances()

	b, err := json.Marshal(a)
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
	arn := "arn:aws:sns:us-west-1:362026750810:binance-account"

	result, err := svc.Publish(&sns.PublishInput{
		Message:  &msg,
		TopicArn: &arn,
	})
	if err != nil {
		log.Fatal(err.Error())
	}

	log.Println(*result.MessageId)
}
