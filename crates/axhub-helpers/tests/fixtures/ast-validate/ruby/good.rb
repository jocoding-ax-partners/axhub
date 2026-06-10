module Sample
  # owner-scoped 테이블은 무필터 list/count 가 정당해요.
  def self.load_posts(client, owner_id)
    mine = client.table("posts").list()
    total = client.table("posts").count()
    filtered = client.table("posts").eq("owner_id", owner_id).limit(20).list()
    [mine, total, filtered]
  end
end
