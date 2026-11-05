#!/usr/bin/env bash
# Deploys the echo API, records vectors against it, and removes it.
#
# Usage: AWS_PROFILE=<profile> conformance/live.sh <deploy|record|probe|teardown|all> [args]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
stack="${APPSYNC_CONFORMANCE_STACK:-appsync-scalars-conformance}"
region="${AWS_REGION:-us-east-1}"
# CI sets this so CloudFormation manages the stack with its own role.
role_args=(${APPSYNC_CONFORMANCE_DEPLOY_ROLE:+--role-arn "$APPSYNC_CONFORMANCE_DEPLOY_ROLE"})

stack_output() {
  aws cloudformation describe-stacks --region "$region" --stack-name "$stack" \
    --query "Stacks[0].Outputs[?OutputKey=='$1'].OutputValue" --output text
}

recorder() {
  APPSYNC_URL="$(stack_output GraphQLUrl)" APPSYNC_API_KEY="$(stack_output ApiKey)" \
    cargo run --quiet --release --manifest-path "$here/recorder/Cargo.toml" -- "$@"
}

deploy() {
  aws cloudformation deploy --region "$region" --stack-name "$stack" \
    --template-file "$here/infra/echo-api.yaml" --no-fail-on-empty-changeset ${role_args[@]+"${role_args[@]}"}
}

record() {
  recorder record --out "$here/vectors" --region "$region" --recorded-at "$(date -u +%F)" "$@"
}

teardown() {
  aws cloudformation delete-stack --region "$region" --stack-name "$stack" ${role_args[@]+"${role_args[@]}"}
  aws cloudformation wait stack-delete-complete --region "$region" --stack-name "$stack"
}

command="${1:-}"
[ $# -gt 0 ] && shift
case "$command" in
  deploy) deploy ;;
  record) record "$@" ;;
  probe) recorder probe "$@" ;;
  teardown) teardown ;;
  all)
    trap teardown EXIT
    deploy
    record "$@"
    ;;
  *)
    echo "usage: $0 <deploy|record|probe|teardown|all> [args]" >&2
    exit 2
    ;;
esac
