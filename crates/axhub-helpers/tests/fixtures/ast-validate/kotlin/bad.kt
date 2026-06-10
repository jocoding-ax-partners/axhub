package sample

// 잘못된 SDK 사용 (block 룰 검출 대상).
fun loadPosts(client: Client) {
    val rows = client.table("posts").or(eq("a", 1), eq("b", 2)).list()
    val n = client.table("posts").not(eq("a", 1)).count()
    val after = "cursor"
    val page = client.table("posts").listAfter(after)
    println("$rows $n $page")
}
