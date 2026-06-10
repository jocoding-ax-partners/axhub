from axhub import db


def load_posts(client):
    # keyset 커서(after=)는 지원 안 해요.
    rows = db.table("posts").list(after="cursor123")
    # pushable 하지 않은 or / not 조합이에요.
    flag = (client.a) or (client.b)
    skip = not (client.c)
    total = db.table("posts").count()
    return rows, flag, skip, total
