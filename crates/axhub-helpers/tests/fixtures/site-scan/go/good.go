package sample

func load(client *Client, ownerID string) {
	posts := client.table("posts").eq("owner_id", ownerID).limit(20).list()
	total := client.table("posts").count()
	_ = posts
	_ = total
}
