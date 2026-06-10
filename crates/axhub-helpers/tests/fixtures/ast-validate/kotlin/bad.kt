package sample

// 잘못된 SDK 사용 (block 룰 검출 대상).
fun loadPosts(client: Client) {
    val rows = client.table("posts").where(Ops.or(eq("a", 1), eq("b", 2))).list()
    val n = client.table("posts").where(Ops.not(eq("a", 1))).count()
    val page = client.table("posts").after("cursor").list()
    println("$rows $n $page")
}
