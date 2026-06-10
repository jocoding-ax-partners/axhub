from axhub import db


def load_posts(client):
    # keyset 커서(after=)는 지원 안 해요.
    rows = db.table("posts").list(after="cursor123")
    # pushable 하지 않은 or_ / not_ SDK combinator 예요.
    pair = db.table("posts").or_(db.eq("a", 1), db.eq("b", 2)).list()
    skip = db.table("posts").not_(db.eq("c", 1)).count()
    total = db.table("posts").count()
    return rows, pair, skip, total
