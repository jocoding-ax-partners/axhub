package sample

fun load(client: Client, ownerId: String) {
    val posts = client.table("posts").eq("owner_id", ownerId).limit(20).list()
    val total = client.table("posts").count()
    println("$posts $total")
}
