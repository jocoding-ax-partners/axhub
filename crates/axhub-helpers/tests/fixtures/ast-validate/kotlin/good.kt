package sample

// owner-scoped 테이블은 무필터 list/count 가 정당해요.
fun loadPosts(client: Client, ownerId: String) {
    val mine = client.table("posts").list()
    val total = client.table("posts").count()
    val filtered = client.table("posts").eq("owner_id", ownerId).limit(20).list()
    println("$mine $total $filtered")
}
