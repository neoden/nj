; XML's highlights, after the query of tree-sitter-xml 0.7.0 (MIT), with
; the changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.
;
; A name is one token to the grammar, prefix and all: `xsl:template` is a
; single Name, and so is `xmlns:xsl`. The prefix cannot be coloured apart
; from the rest, so a prefixed name looks like a plain one.

; Elements
;---------

; As in HTML: the name is the tag, the brackets are brackets.
(STag (Name) @tag)
(ETag (Name) @tag)
(EmptyElemTag (Name) @tag)

; Changed: upstream names an attribute a property; here it is an attribute,
; as in HTML, and its value is one string with its quotes.
(Attribute (Name) @attribute)
(Attribute (AttValue) @string)

; Changed: upstream names a reference a constant; here it is an escape,
; as HTML's entity: `&amp;` and `&#169;` are a character spelt another
; way. Inside an attribute's value it wins over the string around it.
(EntityRef) @escape
(CharRef) @escape
(PEReference) @escape

; Changed: upstream names the section's ends headings and its text raw
; markup; here the ends are the construct's own words and the text is
; plain, as text is everywhere else.
(CDSect (CDStart) @keyword)
(CDSect "]]>" @keyword)

(Comment) @comment

; The declaration and processing instructions
;---------------------------------------------

(XMLDecl "xml" @keyword)
(XMLDecl ["version" "encoding" "standalone"] @attribute)
; Changed: upstream names the version a number, the encoding a special
; string and `yes` a boolean, between plain quotes. Here each value is one
; string with its quotes, as an attribute's.
(XMLDecl ["\"" "'"] @string)
(XMLDecl (VersionNum) @string)
(XMLDecl (EncName) @string)
(XMLDecl ["yes" "no"] @string)

; The target is the instruction's word; what follows it is the target's
; business, plain here.
(PI (PITarget) @keyword)
(StyleSheetPI "xml-stylesheet" @keyword)
(XmlModelPI "xml-model" @keyword)
(PseudoAtt (Name) @attribute)
(PseudoAtt (PseudoAttValue) @string)

; The document type and its internal subset
;--------------------------------------------

; Changed: upstream names the root element a type; it is an element's
; name, as every element name in the subset below.
(doctypedecl "DOCTYPE" @keyword)
(doctypedecl (Name) @tag)

["SYSTEM" "PUBLIC"] @keyword
; Changed: upstream names the URI inside the quotes a link; here each
; literal is one string with its quotes.
(SystemLiteral) @string
(PubidLiteral) @string

(elementdecl "ELEMENT" @keyword)
(elementdecl (Name) @tag)
; The children an element may have, and the words for having any or none.
(Mixed (Name) @tag)
(children (Name) @tag)
["#PCDATA" "EMPTY" "ANY"] @type.builtin

(AttlistDecl "ATTLIST" @keyword)
(AttlistDecl (Name) @tag)
(AttDef (Name) @attribute)
[
  (StringType)
  (TokenizedType)
] @type.builtin
(NotationType "NOTATION" @type.builtin)
(Enumeration (Nmtoken) @string)
; Changed: upstream names these attributes; they are the declaration's
; own words, as `#PCDATA` is.
["#REQUIRED" "#IMPLIED" "#FIXED"] @keyword
(DefaultDecl (AttValue) @string)

(GEDecl "ENTITY" @keyword)
(GEDecl (Name) @constant)
(PEDecl "ENTITY" @keyword)
(PEDecl (Name) @constant)
(EntityValue) @string
(NDataDecl "NDATA" @keyword)
(NDataDecl (Name) @constant)
(NotationDecl "NOTATION" @keyword)
(NotationDecl (Name) @constant)

; Punctuation
;------------

[
  "<"
  ">"
  "</"
  "/>"
  "<?"
  "?>"
  "<!"
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

; The `[` that ends `<![CDATA[` is a token of its own; the start reads
; as one word.
(CDStart "[" @keyword)

[
  ","
  "|"
] @punctuation.delimiter

[
  "="
  "%"
  "*"
  "?"
  "+"
] @operator
