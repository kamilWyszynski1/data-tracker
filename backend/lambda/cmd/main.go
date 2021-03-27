package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/aws/aws-lambda-go/lambda"
	"github.com/binanceBot/backend/binance"
	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"
)

func main() {
	lambda.Start(LambdaHandler)
}

func LambdaHandler() {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	mongoUser := envOrFatal("MONGO_USER")
	mongoPasswd := envOrFatal("MONGO_PASSWD")
	mongoDatabase := envOrFatal("MONGO_DB")
	mongoColl := envOrFatal("MONGO_COLL")
	binanceApiKey := envOrFatal("BINANCE_API_KEY")
	binanceSecretKey := envOrFatal("BINANCE_SECRET_KEY")

	client, err := mongo.Connect(ctx, options.Client().ApplyURI(
		fmt.Sprintf(
			`mongodb+srv://%s:%s@mongo-learning-cluster.skkzi.mongodb.net/%s?retryWrites=true&w=majority`,
			mongoUser, mongoPasswd, mongoDatabase,
		),
	))
	if err != nil {
		log.Fatal(err)
	}
	coll := client.Database(mongoDatabase).Collection(mongoColl)
	bCli := binance.NewBinance(http.DefaultClient, "https://api.binance.com", binanceApiKey, []byte(binanceSecretKey))

	a, err := bCli.Account(binance.AccountRequest{Timestamp: time.Now().Add(-time.Second)})
	if err != nil {
		log.Fatalf("cannot get account info, %s", err)
	}
	_, err = coll.InsertOne(ctx, &binance.AccountResponseWithTimestamp{*a, time.Now()})
	if err != nil {
		log.Fatalf("cannot insert account info, %s", err)
	}
}

func envOrFatal(key string) string {
	if v := os.Getenv(key); v == "" {
		log.Fatalf("%s is empty", key)
	} else {
		return v
	}
	return ""
}
