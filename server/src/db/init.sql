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

create table if not exists service
(
    service_id   varchar(100) not null,
    namespace_id varchar(100) not null,
    meta         varchar(5000),
    create_time  timestamp    not null,
    update_time  timestamp    not null,
    primary key (namespace_id, service_id)
);

insert or ignore into namespace (id, name, description, create_time, update_time)
values ('public', 'public', '保留空间', current_timestamp, current_timestamp);