user.errors.add(:name, 'msg')

user.errors.delete(:name)

user.errors.add(:name, 'msg')

user.errors.attribute_names

user.errors.values

user.errors.to_h

user.errors.to_xml

user.errors[:name] = []

user.errors.messages[:name] = []

user.errors.details[:name] << {}

user.errors.delete(:name)

user.errors[:name].push('msg')

@record.errors.add(:name, 'msg')

user.errors[:name].concat(['msg'])
