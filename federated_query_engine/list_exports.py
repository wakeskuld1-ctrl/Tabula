import pefile

pe = pefile.PE("yascli.dll")
for exp in pe.DIRECTORY_ENTRY_EXPORT.symbols:
    print(exp.name.decode('utf-8'))
