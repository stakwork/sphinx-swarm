## How to backup and restore Neo4j on swarm

#### Backing up Neo4j data

1). connect to your ec2 instance running swarm
2). run the following command `cat ./vol/stack/config.yaml`
3). You should see the following in your `config.yaml`
    ```
    backup_services:
    - boltwall
    - neo4j
    ```
4). If you saw the the `backup_services` then you are fine to continue otherwise
you need to add them to your config and restart the swarm to reflect those in the config.
5). If all is working correctly you should see Neo4j and boltwall backed up to s3 under `s3/buckets/sphinx-swarm/swarm{swarmid}`

#### Restoring neo4j to a previous version

1). Stop neo4j
    `docker stop neo4j.sphinx`
2). Clone data from s3 to ec2 instance

`aws s3 cp s3://sphinx-swarm/{swarm_name}/{backup_date}/ ./backup-{backup_date} --recursive`

Example
`aws s3 cp s3://sphinx-swarm/swarmjualsJ/2026-02-03/ ./backup-2026-02-03 --recursive`
    
3). Backup exisitng data on ec2 instance
`sudo mv /var/lib/docker/volumes/neo4j.sphinx/_data ./neo4j_backup_{todays_date}`

4). Move cloned data to where previous data was
first untar the .tar neo4j file
`tar -xf neo4j.tar`

Now move the backed up data
`sudo mv data/ /var/lib/docker/volumes/neo4j.sphinx/_data`

5). start up neo4j
`docker start neo4j.sphinx`

6). View the logs
`docker logs neo4j.sphinx -f`
