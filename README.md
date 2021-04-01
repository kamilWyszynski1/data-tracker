# BinanceBot

BinanceBot is an application that uses Binance API to track various asspect of your crypto portfolio.

## Used technologies:
* Binance API
* Google Sheets Golang SDK
* [**AWS SNS**](https://aws.amazon.com/sns/?whats-new-cards.sort-by=item.additionalFields.postDateTime&whats-new-cards.sort-order=desc)
* [**AWS LAMBDA**](https://aws.amazon.com/lambda/)
* [**TERRAFORM**](https://www.terraform.io/)
* [**MONGODB ATLAS**](https://www.mongodb.com/cloud/atlas)
* AWS VPC for a static IP for Lambda

## Functionality ideas:
* Earn/Loss per trade
* Push notification when price changes 5/10%
* Pull information from other sources (bank account & stock investing platform)

## Architecture:
* Lambdas for retrieving data from BinanceAPI & inserting it to MongoDB cloud
* SNS for passing data from one lambda to another
* Cloudwatch Event for triggering lambda periodically
* VPC (NAT & Gateway) for providing static IP for lambadas

## TODO
* [ ] add a static IP to lambda so Network Access in MongoAtlas can be configured
* [x] separate calling BinanceAPI from inserting data
* [ ] checkout `provisioner(local-exec)` for a lambda terraform

## Do before
* Generate API KEY from Binance
* Configure aws cli in order to use terraform
* Create MongoAtlas project