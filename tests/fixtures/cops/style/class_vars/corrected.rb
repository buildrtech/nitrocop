class A
  @test = 10
end

class B
  @count = 0
end

class C
  @name = "test"
end

@username, @password = @@ccm_cluster.enable_authentication

@server_cert, @client_cert, @private_key, @passphrase = @@ccm_cluster.enable_ssl_client_auth

@choices, @rest = Parser.parse(@@options, @@args)

@warden_config, @warden_config_blocks = c, b

_port, @remote_ip = Socket.unpack_sockaddr_in(get_peername)

@shard1, @shard2 = TestHelper.recreate_persistent_test_shards

@extended_fields, @topic_types = [], []

@prev, @i = nil, 0

(@a, @b), @c = foo
