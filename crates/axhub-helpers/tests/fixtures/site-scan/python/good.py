from axhub import db


def load(owner_id):
    posts = db.table("posts").eq("owner_id", owner_id).limit(20).list()
    total = db.table("posts").count()
    return posts, total
