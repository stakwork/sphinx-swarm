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
          if (body.port_based_ssl) {
            scripts.push(
              `docker-compose -f second-brain-2.yml up sphinx-swarm -d`
            );
          } else {
            scripts.push(
              `docker-compose -f second-brain.yml up sphinx-swarm -d`
            );
          }
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
      if (url === "/restart-super-admin") {
        const body = await readBody(req);
        if (body.password !== process.env.PASSWORD) {
          return failure(res, "wrong password");
        }
        const scripts = [
          `docker pull sphinxlightning/sphinx-swarm-superadmin`,
          `docker stop sphinx-swarm-superadmin`,
          `docker rm sphinx-swarm-superadmin`,
          `docker-compose -f superadmin.yml up sphinx-swarm-superadmin -d`,
        ];
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
      if (url === "/renew-cert") {
        const body = await readBody(req);
        if (body.password !== process.env.PASSWORD) {
          return failure(res, "wrong password");
        }

        if (process.env.IS_SUPER_ADMIN !== "true") {
          return failure(res, "unauthorized!");
        }

        const CERT_EMAIL = process.env.CERT_EMAIL;

        if (!CERT_EMAIL) {
          return failure(res, "invalid cert email");
        }

        try {
          const script = `
            sudo certbot certonly \
            --dns-route53 \
            --email ${CERT_EMAIL} \
            --agree-tos \
            --expand \
            --non-interactive \
            --force-renewal \
            -d "*.sphinx.chat" \
            -d "sphinx.chat"
          `;
          const { stdout, stderr } = await exec(script);
          console.log(stdout);
          console.log("error:", stderr);
          respond(res, { ok: true, message: stdout, error: stderr });
        } catch (error) {
          console.log("error:", e);
          failure(res, e.message);
        }
      }
      if (url === "/upload-cert") {
        const body = await readBody(req);
        if (body.password !== process.env.PASSWORD) {
          return failure(res, "wrong password");
        }

        if (process.env.IS_SUPER_ADMIN !== "true") {
          return failure(res, "unauthorized!");
        }
        const CERT_BUCKET = process.env.CERT_BUCKET;
        if (!CERT_BUCKET) {
          return failure(res, "cert bucket not provided!");
        }
        try {
          const message = [];
          const errMsg = [];
          const scripts = [
            `sudo rm -rf /home/admin/certs`,
            `sudo rm -f /home/admin/data.zip`,
            `sudo mkdir -p /home/admin/certs`,
            `sudo cp /etc/letsencrypt/live/sphinx.chat/fullchain.pem /home/admin/certs/sphinx.chat.crt`,
            `sudo cp /etc/letsencrypt/live/sphinx.chat/privkey.pem /home/admin/certs/sphinx.chat.key`,
            `sudo cp /home/admin/tls.yml /home/admin/certs/tls.yml`,
            `sudo zip -r /home/admin/data.zip /home/admin/certs/`,
            `aws s3 cp /home/admin/data.zip s3://${CERT_BUCKET}/data.zip`,
          ];

          for (const sc of scripts) {
            const { stdout, stderr } = await exec(sc);
            console.log(stdout);
            message.push(stdout);
            console.log("error:", stderr);
            errMsg.push(stderr);
          }
          respond(res, {
            ok: true,
            message: message.join(","),
            error: errMsg.join(","),
          });
        } catch (error) {
          console.log("error:", e);
          failure(res, e.message);
        }
      }
      if (url === "/update-ssl-cert") {
        const body = await readBody(req);
        if (body.password !== process.env.PASSWORD) {
          return failure(res, "wrong password");
        }

        const aws_s3_cert_bucket_name = body.cert_bucket_name;

        if (!aws_s3_cert_bucket_name) {
          return failure(res, "Please provide valid bucket name");
        }
        const scripts = [
          `sudo rm -f /home/admin/data.zip`,
          `aws s3 cp s3://${aws_s3_cert_bucket_name}/data.zip /home/admin/`,
          `docker stop load_balancer`,
          `docker rm load_balancer`,
          `sudo rm -rf /home/admin/certs`,
          `sudo mkdir -p /home/admins/certs`,
          `sudo unzip -o -j /home/admin/data.zip -d /home/admin/certs/`,
          `sudo chown admin:admin /home/admin/certs/*`,
          `sudo chmod 644 /home/admin/certs/sphinx.chat.crt`,
          `sudo chmod 600 /home/admin/certs/sphinx.chat.key`,
          `docker-compose -f second-brain-2.yml up load_balancer -d`,
        ];
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
