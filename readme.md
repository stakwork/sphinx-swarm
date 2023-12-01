# sphinx-swarm

![Swarm](https://github.com/stakwork/sphinx-swarm/raw/master/testscripts/sphinx-swarm.png)

### stack

`cargo run --bin stack`

`cd app`

`yarn dev`

[http://localhost:5173/](http://localhost:5173/)

login with `admin`/`password`

### pull nodes down

`./clear.sh`


### Restart Swarm
If swarm goes down.

1. Find ec2 instance on aws (ex. sphinx-swarm-19) and reboot instance (reboot only if you can't SSH into it)
2. ssh into instance
3. `cd sphinx-swarm`
4. `./stop jarvis` (optional if restarting just jarvis-backend)
5. `./restart.sh`
