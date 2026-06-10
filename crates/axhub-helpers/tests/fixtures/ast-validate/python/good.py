from axhub import db


def load_posts(client, owner_id):
    # owner-scoped 테이블은 무필터 list/count 가 정당해요.
    mine = db.table("posts").list()
    total = db.table("posts").count()
    filtered = db.table("posts").eq("owner_id", owner_id).limit(20).list()
    return mine, total, filtered
