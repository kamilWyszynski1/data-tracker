package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"

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

		var data interface{}
		if err := json.Unmarshal([]byte(snsRecord.Message), &data); err != nil {
			log.Fatalf("failed to unrmashl message to bson.D, %s", err)
		}

		log.Println("mongoDB connected")
		sub := snsRecord.Subject
		split := strings.Split(sub, ":")

		coll := client.Database(split[0]).Collection(split[1])

		_, err = coll.InsertOne(ctx, data)
		if err != nil {
			log.Fatalf("cannot insert account info, %s", err)
		}
	}
}

func main() {
	lambda.Start(handler)
}
