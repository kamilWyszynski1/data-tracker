resource "aws_s3_bucket" "binance-lambda" {
  bucket = "binance-lambda-version1"
  acl    = "private"
}

variable "app_version" {default = "0.0.1"}

resource "aws_s3_bucket_object" "binance-lambda-code" {
  bucket = aws_s3_bucket.binance-lambda.bucket
  key    = "${var.app_version}code.zip"

  source = "../backend/lambda/code.zip"
  etag   = "../backend/lambda/code.zip"
}

resource "aws_iam_role" "iam_for_lambda" {
  name = "iam_for_lambda"

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

resource "aws_lambda_function" "binance-lambda" {
  function_name = "binance-lambda-ver1"

  s3_bucket     = aws_s3_bucket_object.binance-lambda-code.bucket
  s3_key        = aws_s3_bucket_object.binance-lambda-code.key
  s3_object_version = aws_s3_bucket_object.binance-lambda-code.version_id

  role          = aws_iam_role.iam_for_lambda.arn
  handler       = "main"

  runtime = "go1.x"


  environment {
    variables = {
      MONGO_USER         = "atlasAdmin"
      MONGO_PASSWD       = "WQuBjhoA3pkxNGBY"
      MONGO_DB           = "test"
      MONGO_COLL         = "test4"
      BINANCE_API_KEY    = "QbLjiZkYn6mReDrK8wI64uKh2GF42F2ezmigik7prdH212Yi5I5f3wRCTbVWWktm"
      BINANCE_SECRET_KEY = "5BiEPBLveIXIrNqLX9hqV0QAvYZYNK3TVALk6ZBEdrpBsBGYVl2zeQhDEDZa4jUB"
    }
  }
}

resource "aws_cloudwatch_event_rule" "binance-lambda-cron" {
  name = "binance-lambda-run"
  description = "Run binance-lambda periodically"

  schedule_expression = "rate(24 hours)"
}

resource "aws_cloudwatch_event_target" "binance-lambda-cron-target" {
  arn = aws_lambda_function.binance-lambda.arn
  rule = aws_cloudwatch_event_rule.binance-lambda-cron.name
}

// we need to allow CloudWatch to trigger lambda
resource "aws_lambda_permission" "cloudwatch-perrmision-for-lambda" {
  statement_id = "AllowExecutionFromCloudWatch"
  action = "lambda:InvokeFunction"
  function_name = aws_lambda_function.binance-lambda.function_name
  principal = "events.amazonaws.com"
  source_arn = aws_cloudwatch_event_rule.binance-lambda-cron.arn
}