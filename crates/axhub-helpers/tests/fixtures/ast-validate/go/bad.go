package sample

type listOpts struct {
	after  string
	before string
}

// 잘못된 SDK 사용 (block 룰 검출 대상).
func loadPosts(client *Client) {
	rows := client.table("posts").or(eq("a", 1), eq("b", 2)).list()
	n := client.table("posts").not(eq("a", 1)).count()
	page := client.table("posts").listWith(listOpts{after: "cursor"})
	_ = rows
	_ = n
	_ = page
}
