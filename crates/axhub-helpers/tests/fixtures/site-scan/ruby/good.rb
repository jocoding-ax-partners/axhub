def load(client, owner_id)
  posts = client.table("posts").eq("owner_id", owner_id).limit(20).list()
  total = client.table("posts").count()
  [posts, total]
end
