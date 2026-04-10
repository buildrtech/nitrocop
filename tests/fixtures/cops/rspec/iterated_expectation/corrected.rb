expect([user1, user2, user3]).to all(be_valid)
expect([item1, item2]).to all(be_a(Item))
expect(users).to all(be_valid)
