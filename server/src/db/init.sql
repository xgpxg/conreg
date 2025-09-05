create table if not exists config
(
    id_          integer primary key,
    namespace_id varchar(100) not null,
    id           varchar(500) not null,
    content      text         not null,
    create_time  timestamp    not null,
    update_time  timestamp    not null,
    description  varchar(500),
    md5          varchar(32)  not null,
    unique (namespace_id, id)
);
create table if not exists config_history
(
    id_          integer primary key,
    namespace_id varchar(100) not null,
    id           varchar(500) not null,
    content      text         not null,
    create_time  timestamp    not null,
    update_time  timestamp    not null,
    description  varchar(500),
    md5          varchar(32)  not null
);

create table if not exists namespace
(
    id          varchar(100) primary key,
    name        varchar(100) not null,
    description varchar(500),
    create_time timestamp    not null,
    update_time timestamp    not null
);