package sample;

// 잘못된 SDK 사용 (block 룰 검출 대상).
class Sample {
    void loadPosts(Client client) {
        var rows = client.table("posts").or(eq("a", 1), eq("b", 2)).list();
        var n = client.table("posts").not(eq("a", 1)).count();
        String after = "cursor";
        var page = client.table("posts").listAfter(after);
        System.out.println(rows + n + page);
    }
}
