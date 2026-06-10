from axhub import db


def load_posts(client, owner_id):
    # owner-scoped 테이블은 무필터 list/count 가 정당해요.
    mine = db.table("posts").list()
    total = db.table("posts").count()
    filtered = db.table("posts").eq("owner_id", owner_id).limit(20).list()
    # boolean 키워드 near-miss — SDK or_()/not_() 가 아니라 통과해야 해요.
    flag = (client.a) or (client.b)
    skip = not (client.c)
    return mine, total, filtered, flag, skip
