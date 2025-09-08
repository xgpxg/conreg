# Conreg Cluster Manage Tool

conreg-cmt is a management tool provided for conreg clusters, used for cluster creation, scaling up, and scaling down.

```shell
Usage: conreg-cmt --server <SERVER> <COMMAND>

Commands:
  init         Initialize the cluster
  add-learner  Add a learner node to the cluster
  promote      Promote some learner node to a full member, must call "add-learner" first
  remove-node  Remove a node from the cluster
  status       Get cluster status
  monitor      Monitor cluster status
  help         Print this message or the help of the given subcommand(s)

Options:
  -s, --server <SERVER>  Address of any node in the cluster [default: 127.0.0.1:8000]
  -h, --help             Print help
  -V, --version          Print version
```

# Examples

- Initialize the cluster (a node must be started first)

```shell
conreg-cmt -s 127.0.0.1:8001 init 1=127.0.0.1:8001 2=127.0.0.1:8002 3=127.0.0.1:8003
```

- Add a learner node

```shell
conreg-cmt -s 127.0.0.1:8001 add-learner 4=127.0.0.1:8004
```

- Promote learner node to member node

```shell
conreg-cmt -s 127.0.0.1:8001 promote 4
```

- Remove a node

```shell
conreg-cmt -s 127.0.0.1:8001 remove-node 4
```

- Monitor cluster status

```shell
conreg-cmt -s 127.0.0.1:8001 monitor
```

You might get the following result:

```text
┌────────────────────────────────────────────────────────────────┐
│                        Cluster Status                          │
├────────────────────────────────────────────────────────────────┤
│ Current Node ID               : 1                              │
│ Current Node Status           : Leader                         │
│ Current Node Term             : 9                              │
│ Leader                        : 1                              │
│ Last Log Index                : 41                             │
│ Last Applied Index            : 41                             │
│ Communication delay           : 1 ms                           │
│                                                                │
│ Members:                                                       │
│   - Node 1                    : 127.0.0.1:8000                 │
│   - Node 2                    : 127.0.0.1:8001                 │
│   - Node 3                    : 127.0.0.1:8002                 │
│                                                                │
│ Replication:                                                   │
│   - Node 1                    : Index 41                       │
│   - Node 2                    : Index 41                       │
│   - Node 3                    : Index 41                       │
└────────────────────────────────────────────────────────────────┘
```