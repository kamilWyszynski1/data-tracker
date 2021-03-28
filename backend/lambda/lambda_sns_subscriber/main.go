package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/binanceBot/backend/binance"

	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"

	"github.com/aws/aws-lambda-go/events"
	"github.com/aws/aws-lambda-go/lambda"
	"github.com/binanceBot/backend/lambda/common"
)

var (
	mongoUser     = common.EnvOrFatal("MONGO_USER")
	mongoPasswd   = common.EnvOrFatal("MONGO_PASSWD")
	mongoDatabase = common.EnvOrFatal("MONGO_DB")
	mongoColl     = common.EnvOrFatal("MONGO_COLL")
)

func handler(ctx context.Context, snsEvent events.SNSEvent) {
	for _, record := range snsEvent.Records {
		snsRecord := record.SNS

		log.Printf("[%s %s] Message = %s \n", record.EventSource, snsRecord.Timestamp, snsRecord.Message)

		client, err := mongo.Connect(ctx, options.Client().ApplyURI(
			fmt.Sprintf(
				`mongodb+srv://%s:%s@mongo-learning-cluster.skkzi.mongodb.net/%s?retryWrites=true&w=majority`,
				mongoUser, mongoPasswd, mongoDatabase,
			),
		))
		if err != nil {
			log.Fatal(err)
		}
		log.Println("mongoDB connected")

		coll := client.Database(mongoDatabase).Collection(mongoColl)

		var ar binance.AccountResponse

		if err := json.NewDecoder(strings.NewReader(snsRecord.Message)).Decode(&ar); err != nil {
			log.Fatal(err)
		}

		_, err = coll.InsertOne(ctx, binance.AccountResponseWithTimestamp{ar, time.Now()})
		if err != nil {
			log.Fatalf("cannot insert account info, %s", err)
		}
	}
}

func main() {
	lambda.Start(handler)
}
