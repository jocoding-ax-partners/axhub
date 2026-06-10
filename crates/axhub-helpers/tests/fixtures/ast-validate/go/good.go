package sample

// owner-scoped 테이블은 무필터 list/count 가 정당해요.
func loadPosts(client *Client, ownerID string) {
	mine := client.table("posts").list()
	total := client.table("posts").count()
	filtered := client.table("posts").eq("owner_id", ownerID).limit(20).list()
	_ = mine
	_ = total
	_ = filtered
}
