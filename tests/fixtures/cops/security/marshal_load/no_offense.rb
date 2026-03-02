Marshal.dump(data)
obj.load(data)
marshal_load(data)
Marshal.dump(obj)
x = Marshal
Marshal.load(Marshal.dump(data))
Marshal.load(Marshal.dump({}))
Marshal.restore(Marshal.dump(obj))
Marshal.load
Marshal.restore
::Marshal.load(::Marshal.dump(data))
::Marshal.restore(::Marshal.dump(data))
