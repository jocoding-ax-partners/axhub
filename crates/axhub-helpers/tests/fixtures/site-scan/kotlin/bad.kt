package sample

import java.net.http.HttpClient
import java.sql.DriverManager

fun load() {
    val http = HttpClient.newHttpClient()
    val conn = DriverManager.getConnection("jdbc:postgresql://db/x")
    val url = "https://api.axhub.dev/v1/posts"
    println("$http $conn $url")
}
