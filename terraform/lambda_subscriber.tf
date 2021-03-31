resource "aws_s3_bucket" "binance_lambda_subscriber" {
  bucket = "binance-lambda-subscriber"
  acl    = "private"
}

variable "subscriber_version" { default = "0.0.7" }

resource "aws_s3_bucket_object" "binance_lambda_subscriber_code" {
  bucket = aws_s3_bucket.binance_lambda_subscriber.bucket
  key    = "${var.subscriber_version}code.zip"

  source = "../backend/lambda/lambda_sns_subscriber/code.zip"
  etag   = "../backend/lambda/lambda_sns_subscriber/code.zip"

  depends_on = [
    aws_s3_bucket.binance_lambda_subscriber
  ]
}

resource "aws_iam_role" "iam_for_lambda_subscriber" {
  name = "iam_for_lambda_subscriber"

  assume_role_policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": "sts:AssumeRole",
      "Principal": {
        "Service": "lambda.amazonaws.com"
      },
      "Effect": "Allow",
      "Sid": ""
    }
  ]
}
EOF
}

variable "mongo_user" {}
variable "mongo_passwd" {}
variable "mongo_db" {}
variable "mongo_coll" {}

resource "aws_lambda_function" "binance_lambda_subscriber" {
  function_name = "binance_lambda_subscriber"

  s3_bucket         = aws_s3_bucket_object.binance_lambda_subscriber_code.bucket
  s3_key            = aws_s3_bucket_object.binance_lambda_subscriber_code.key
  s3_object_version = aws_s3_bucket_object.binance_lambda_subscriber_code.version_id

  role    = aws_iam_role.iam_for_lambda_subscriber.arn
  handler = "main"

  runtime = "go1.x"
  timeout = 15




  environment {
    variables = {
      MONGO_USER    = var.mongo_user
      MONGO_PASSWD = var.mongo_passwd
      MONGO_DB = var.mongo_db
      MONGO_COLL = var.mongo_coll
    }
  }

  depends_on = [
    aws_s3_bucket_object.binance_lambda_subscriber_code,
    aws_iam_role.iam_for_lambda_subscriber,

  ]
}
/*
 LOGGING
*/

resource "aws_iam_role_policy_attachment" "lambda_subscriber_logs" {
  role       = aws_iam_role.iam_for_lambda_subscriber.name
  policy_arn = aws_iam_policy.lambda_logging.arn
}


/*
 SNS
*/

// create subscription, link topic with lambda
resource "aws_sns_topic_subscription" "topic_lambda_subscriber" {
  endpoint = aws_lambda_function.binance_lambda_subscriber.arn
  protocol = "lambda"
  topic_arn = aws_sns_topic.binance-publishes.arn
}

// allow execution
resource "aws_lambda_permission" "lambda_subscriber_with_sns" {
  action = "lambda:InvokeFunction"
  function_name = aws_lambda_function.binance_lambda_subscriber.function_name
  principal = "sns.amazonaws.com"
  source_arn = aws_sns_topic.binance-publishes.arn
  statement_id = "AllowExecutionFromSNS"
}

/*
 VPC
*/

resource "aws_iam_role_policy_attachment" "lambda_subscriber_vpc_access" {
  role       = aws_iam_role.iam_for_lambda_subscriber.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole"
}