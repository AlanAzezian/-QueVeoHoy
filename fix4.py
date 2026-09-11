import codecs

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

bad_snippet = '''                </button>
              )}
            </div>
          </div>
        </div>
      )}'''

good_snippet = '''                </button>
              )}
          </div>
        </div>
      )}'''

if bad_snippet in content:
    content = content.replace(bad_snippet, good_snippet)
    with codecs.open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("Fixed extra div!")
else:
    print("Could not find bad snippet.")
