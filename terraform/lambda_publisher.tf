resource "aws_s3_bucket" "binance_lambda_publisher" {
  bucket = "binance-lambda-publisher"
  acl    = "private"
}

variable "publisher_version" { default = "0.0.2" }

resource "aws_s3_bucket_object" "binance_lambda_publisher_code" {
  bucket = aws_s3_bucket.binance_lambda_publisher.bucket
  key    = "${var.publisher_version}code.zip"

  source = "../backend/lambda/lambda_sns_publisher/code.zip"
  etag   = "../backend/lambda/lambda_sns_publisher/code.zip"

  depends_on = [
    aws_s3_bucket.binance_lambda_publisher
  ]
}

resource "aws_iam_role" "iam_for_lambda_publisher" {
  name = "iam_for_lambda_publisher"

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

variable "binance_api_key" {}
variable "binance_secret_key" {}

resource "aws_lambda_function" "binance_lambda_publisher" {
  function_name = "binance_lambda_publisher"

  s3_bucket         = aws_s3_bucket_object.binance_lambda_publisher_code.bucket
  s3_key            = aws_s3_bucket_object.binance_lambda_publisher_code.key
  s3_object_version = aws_s3_bucket_object.binance_lambda_publisher_code.version_id

  role    = aws_iam_role.iam_for_lambda_publisher.arn
  handler = "main"

  runtime = "go1.x"
  timeout = 15


  environment {
    variables = {
      BINANCE_API_KEY    = var.binance_api_key
      BINANCE_SECRET_KEY = var.binance_secret_key
    }
  }

  depends_on = [
    aws_s3_bucket_object.binance_lambda_publisher_code,
    aws_iam_role.iam_for_lambda_publisher,

  ]
}

resource "aws_cloudwatch_event_rule" "binance_lambda_publisher_cron" {
  name        = "binance_lambda_run"
  description = "Run binance_lambda periodically"

  schedule_expression = "rate(24 hours)"
}

resource "aws_cloudwatch_event_target" "binance_lambda_cron_target" {
  arn  = aws_lambda_function.binance_lambda_publisher.arn
  rule = aws_cloudwatch_event_rule.binance_lambda_publisher_cron.name
}

// we need to allow CloudWatch to trigger lambda
resource "aws_lambda_permission" "cloudwatch_permission_for_lambda" {
  statement_id  = "AllowExecutionFromCloudWatch"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.binance_lambda_publisher.function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.binance_lambda_publisher_cron.arn
}

/*
 LOGGING
*/

resource "aws_cloudwatch_log_group" "default" {
  name              = "/default"
  retention_in_days = 7
}

resource "aws_iam_policy" "lambda_logging" {
  name        = "lambda_logging"
  path        = "/"
  description = "IAM policy for logging from a lambda"

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*",
      "Effect": "Allow"
    }
  ]
}
EOF
}

resource "aws_iam_role_policy_attachment" "lambda_publisher_logs" {
  role       = aws_iam_role.iam_for_lambda_publisher.name
  policy_arn = aws_iam_policy.lambda_publisher_policy.arn
}

/*
  SNS
*/

resource "aws_sns_topic" "binance-publishes" {
  name = "binance-account"
}

resource "aws_iam_policy" "lambda_publisher_policy" {
  name        = "lambda_publisher_policy"
  path        = "/"
  description = "IAM policy for SNS for lambda publisher"

  depends_on = [
    aws_sns_topic.binance-publishes
  ]

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": [
        "sns:Publish"
      ],
      "Resource": "${aws_sns_topic.binance-publishes.arn}",
      "Effect": "Allow"
    }
  ]
}
EOF
}

resource "aws_iam_role_policy_attachment" "lambda_publisher_sns" {
  role       = aws_iam_role.iam_for_lambda_publisher.name
  policy_arn = aws_iam_policy.lambda_publisher_policy.arn
}