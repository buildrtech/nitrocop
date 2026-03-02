Marshal.load(data)
        ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
Marshal.restore(data)
        ^^^^^^^ Security/MarshalLoad: Avoid using `Marshal.restore`.
::Marshal.load(x)
          ^^^^ Security/MarshalLoad: Avoid using `Marshal.load`.
::Marshal.restore(x)
          ^^^^^^^ Security/MarshalLoad: Avoid using `Marshal.restore`.
