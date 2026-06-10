package sample;

// owner-scoped 테이블은 무필터 list/count 가 정당해요.
class Sample {
    void loadPosts(Client client, String ownerId) {
        var mine = client.table("posts").list();
        var total = client.table("posts").count();
        var filtered = client.table("posts").eq("owner_id", ownerId).limit(20).list();
        System.out.println(mine + total + filtered);
    }
}
