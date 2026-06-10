module Sample
  # 잘못된 SDK 사용 (block 룰 검출 대상).
  def self.load_posts(client)
    rows = client.table("posts").or_(eq("a", 1), eq("b", 2)).list(after: "cursor")
    n = client.table("posts").not_(eq("a", 1)).count
    [rows, n]
  end
end
