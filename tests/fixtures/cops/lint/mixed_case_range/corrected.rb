r = ('A'..'z')
x = ('a'..'Z')
y = ('B'..'f')

re = /[A-Za-z]/

chars = /[a-zA-Za-z0-9]{0,32}/

regexp = /[#{prefix}A-Za-z#{suffix}]/

POTENTIAL_BYTES = (' '..'z').to_a

PRINTABLE = ("!".."9").to_a + (':'..'Z').to_a + ('['..'z').to_a + ('{'..'~').to_a

chars  = ("\x21".."\x5A").to_a

CHARS = ('0'..'z').to_a
