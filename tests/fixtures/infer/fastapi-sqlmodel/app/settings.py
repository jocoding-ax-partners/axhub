import os

DATABASE_URL = os.getenv("DATABASE_URL")
SECRET_KEY = os.getenv("SECRET_KEY")
PORT = int(os.getenv("PORT", "8000"))


# 코드-only 모델(마이그레이션 없음) — best-effort, 검토 필요 대상
class CartItem:
    id: int
    product_id: int
    quantity: int
