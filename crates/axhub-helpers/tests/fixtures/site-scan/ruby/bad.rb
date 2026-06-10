require "net/http"
require "pg"

def load
  res = Net::HTTP.get(URI("https://api.axhub.dev/v1/posts"))
  conn = PG.connect(dbname: "x")
  url = "https://backend.example.com/data"
  [res, conn, url]
end
