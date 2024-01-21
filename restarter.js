const http = require("http");
const util = require("util");
const exec = util.promisify(require("child_process").exec);

async function yo(_req, res) {
  respond(res, { hi: "hello" });
}

const hostname = "0.0.0.0";
const port = process.env.PORT ? parseInt(process.env.PORT) : 3003;

function is2b() {
  return (
    process.env.SECOND_BRAIN === "true" || process.env.SECOND_BRAIN === "1"
  );
}

http
  .createServer(async (req, res) => {
    if (req.method === "OPTIONS") {
      return end(res, 200, "");
    }
    console.log("=>", req.url, req.method);
    if (!req.url) return console.error("no url");

    const url = req.url.split("?")[0];

    if (req.method === "GET") {
      console.log(url);
      if (url === "/yo") {
        yo(req, res);
      }
    }
    if (req.method === "POST") {
      if (url === "/restart") {
        const body = await readBody(req);
        if (body.password !== process.env.PASSWORD) {
          return failure(res, "wrong password");
        }
        const scripts = [
          `docker pull sphinxlightning/sphinx-swarm:latest`,
          `docker stop sphinx-swarm`,
          `docker rm sphinx-swarm`,
        ];
        if (is2b()) {
          scripts.push(`docker-compose -f second-brain.yml up sphinx-swarm -d`);
        } else {
          scripts.push(`docker-compose up sphinx-swarm -d`);
        }
        console.log("exec!");
        try {
          for (const sc of scripts) {
            const { stdout, stderr } = await exec(sc);
            console.log(stdout);
            console.log("error:", stderr);
          }
          respond(res, { ok: true });
        } catch (e) {
          console.log("error:", e);
          failure(res, e.message);
        }
      }
    }
  })
  .listen(port, hostname, () => {
    console.log(`Server running at http://${hostname}:${port}/`);
  });

function respond(res, response) {
  end(res, 200, JSON.stringify(response));
}

function failure(res, err_msg) {
  end(res, 401, JSON.stringify({ error: err_msg }));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    req.setEncoding("utf8");
    req.on("data", function (data) {
      try {
        const body = JSON.parse(data);
        resolve(body);
      } catch (e) {
        reject(e);
      }
    });
  });
}

function end(res, status, data) {
  const headers = {};
  headers["Content-Type"] = "application/json";
  headers["Access-Control-Allow-Origin"] = "*";
  headers["Access-Control-Allow-Methods"] = "POST, GET, PUT, DELETE, OPTIONS";
  headers["Access-Control-Allow-Credentials"] = false;
  headers["Access-Control-Max-Age"] = "86400"; // 24 hours
  headers["Access-Control-Allow-Headers"] =
    "X-Requested-With, X-HTTP-Method-Override, Content-Type, Accept";
  res.writeHead(200, headers);
  res.end(data);
}

/*
curl http://localhost:3003/yo

curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"password":"123"}' \
  http://localhost:3003/restart
*/
