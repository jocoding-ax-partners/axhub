import stripe

# NOTE: 이 fixture 는 하드코딩 시크릿 탐지를 데모해요.
# 값은 전부 FAKE_PLACEHOLDER — 실제 시크릿이 아니에요.
stripe.api_key = "sk_test_FAKE_PLACEHOLDER_DO_NOT_USE_0000"

DB_PASSWORD = "hunter2_FAKE_PLACEHOLDER"
