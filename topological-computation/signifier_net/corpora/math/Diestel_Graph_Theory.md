Reinhard Diestel

Graph Theory

Electronic Edition 2000

c

°  Springer-Verlag New York 1997, 2000

This is an electronic version of the second (2000) edition of the above Springer book, from their series  Graduate Texts in Mathematics, vol. 173.

The cross-references in the text and in the margins are active links: click on them to be taken to the appropriate page.

The printed edition of this book can be ordered from your bookseller, or electronically from Springer through the Web sites referred to below.

Softcover $34.95, ISBN 0-387-98976-5

Hardcover $69.95, ISBN 0-387-95014-1

Further information (reviews, errata, free copies for lecturers etc.) and electronic order forms can be found on

http://www.math.uni-hamburg.de/home/diestel/books/graph.theory/

http://www.springer-ny.com/supplements/diestel/

Preface

Almost two decades have passed since the appearance of those graph the-

ory texts that still set the agenda for most introductory courses taught today. The canon created by those books has helped to identify some

main fields of study and research, and will doubtless continue to influence the development of the discipline for some time to come.

Yet much has happened in those 20 years, in graph theory no less

than elsewhere: deep new theorems have been found, seemingly disparate

methods and results have become interrelated, entire new branches have

arisen. To name just a few such developments, one may think of how

the new notion of list colouring has bridged the gulf between invari-

ants such as average degree and chromatic number, how probabilistic

methods and the regularity lemma have pervaded extremal graph theo-

ry and Ramsey theory, or how the entirely new field of graph minors and tree-decompositions has brought standard methods of surface topology

to bear on long-standing algorithmic graph problems.

Clearly, then, the time has come for a reappraisal:  what are, today, the essential areas, methods and results that should form the centre of an introductory graph theory course aiming to equip its audience for the most likely developments ahead?

I have tried in this book to offer material for such a course. In

view of the increasing complexity and maturity of the subject, I have

broken with the tradition of attempting to cover both theory and appli-

cations: this book offers an introduction to the theory of graphs as part of (pure) mathematics; it contains neither explicit algorithms nor ‘real world’ applications. My hope is that the potential for depth gained by

this restriction in scope will serve students of computer science as much as their peers in mathematics: assuming that they prefer algorithms but will benefit from an encounter with pure mathematics of  some  kind, it seems an ideal opportunity to look for this close to where their heart lies!

In the selection and presentation of material, I have tried to ac-

commodate two conflicting goals. On the one hand, I believe that an

viii

Preface

introductory text should be lean and concentrate on the essential, so as to offer guidance to those new to the field. As a graduate text, moreover, it should get to the heart of the matter quickly: after all, the idea is to convey at least an impression of the depth and methods of the subject.

On the other hand, it has been my particular concern to write with

sufficient detail to make the text enjoyable and easy to read: guiding

questions and ideas will be discussed explicitly, and all proofs presented will be rigorous and complete.

A typical chapter, therefore, begins with a brief discussion of what

are the guiding questions in the area it covers, continues with a succinct account of its classic results (often with simplified proofs), and then presents one or two deeper theorems that bring out the full flavour of

that area. The proofs of these latter results are typically preceded by (or interspersed with) an informal account of their main ideas, but are then presented formally at the same level of detail as their simpler counterparts. I soon noticed that, as a consequence, some of those proofs came out rather longer in print than seemed fair to their often beautifully

simple conception. I would hope, however, that even for the professional reader the relatively detailed account of those proofs will at least help to minimize reading time . . .

If desired, this text can be used for a lecture course with little or

no further preparation. The simplest way to do this would be to follow

the order of presentation, chapter by chapter: apart from two clearly

marked exceptions, any results used in the proof of others precede them in the text.

Alternatively, a lecturer may wish to divide the material into an easy

basic course for one semester, and a more challenging follow-up course

for another. To help with the preparation of courses deviating from the order of presentation, I have listed in the margin next to each proof the reference numbers of those results that are used in that proof. These

references are given in round brackets: for example, a reference (4.1.2) in the margin next to the proof of Theorem 4.3.2 indicates that Lemma

4.1.2 will be used in this proof. Correspondingly, in the margin next to Lemma 4.1.2 there is a reference [ 4.3.2 ] (in square brackets) informing the reader that this lemma will be used in the proof of Theorem 4.3.2.

Note that this system applies between different sections only (of the same or of different chapters): the sections themselves are written as units and best read in their order of presentation.

The mathematical prerequisites for this book, as for most graph

theory texts, are minimal: a first grounding in linear algebra is assumed for Chapter 1.9 and once in Chapter 5.5, some basic topological concepts about the Euclidean plane and 3-space are used in Chapter 4, and

a previous first encounter with elementary probability will help with

Chapter 11. (Even here, all that is assumed formally is the knowledge

of basic definitions: the few probabilistic tools used are developed in the

Preface

ix

text.) There are two areas of graph theory which I find both fascinat-

ing and important, especially from the perspective of pure mathematics

adopted here, but which are not covered in this book: these are algebraic graph theory and infinite graphs.

At the end of each chapter, there is a section with exercises and

another with bibliographical and historical notes. Many of the exercises were chosen to complement the main narrative of the text: they illustrate new concepts, show how a new invariant relates to earlier ones,

or indicate ways in which a result stated in the text is best possible.

Particularly easy exercises are identified by the superscript  −, the more challenging ones carry a +. The notes are intended to guide the reader

on to further reading, in particular to any monographs or survey articles on the theme of that chapter. They also offer some historical and other remarks on the material presented in the text.

Ends of proofs are marked by the symbol ¤. Where this symbol is

found directly below a formal assertion, it means that the proof should be clear after what has been said—a claim waiting to be verified! There are also some deeper theorems which are stated, without proof, as background information: these can be identified by the absence of both proof and ¤.

Almost every book contains errors, and this one will hardly be an

exception. I shall try to post on the Web any corrections that become

necessary. The relevant site may change in time, but will always be

accessible via the following two addresses:

http://www.springer-ny.com/supplements/diestel/

http://www.springer.de/catalog/html-files/deutsch/math/3540609180.html

Please let me know about any errors you find.

Little in a textbook is truly original: even the style of writing and

of presentation will invariably be influenced by examples. The book that no doubt influenced me most is the classic GTM graph theory text by

Bollobás: it was in the course recorded by this text that I learnt my first graph theory as a student. Anyone who knows this book well will feel

its influence here, despite all differences in contents and presentation.

I should like to thank all who gave so generously of their time,

knowledge and advice in connection with this book. I have benefited

particularly from the help of N. Alon, G. Brightwell, R. Gillett, R. Halin, M. Hintz, A. Huck, I. Leader, T. ÃLuczak, W. Mader, V. Rödl, A.D. Scott, P.D. Seymour, G. Simonyi, M. Škoviera, R. Thomas, C. Thomassen and

P. Valtr. I am particularly grateful also to Tommy R. Jensen, who taught me much about colouring and all I know about  k-flows, and who invest-ed immense amounts of diligence and energy in his proofreading of the

preliminary German version of this book.

March 1997

RD

x

Preface

About the second edition

Naturally, I am delighted at having to write this addendum so soon after this book came out in the summer of 1997. It is particularly gratifying to hear that people are gradually adopting it not only for their personal use but more and more also as a course text; this, after all, was my aim when I wrote it, and my excuse for agonizing more over presentation

than I might otherwise have done.

There are two major changes. The last chapter on graph minors

now gives a complete proof of one of the major results of the Robertson-Seymour theory, their theorem that excluding a graph as a minor bounds

the tree-width if and only if that graph is planar. This short proof did not exist when I wrote the first edition, which is why I then included a short proof of the next best thing, the analogous result for path-width.

That theorem has now been dropped from Chapter 12. Another addition

in this chapter is that the tree-width duality theorem, Theorem 12.3.9, now comes with a (short) proof too.

The second major change is the addition of a complete set of hints

for the exercises. These are largely Tommy Jensen’s work, and I am

grateful for the time he donated to this project. The aim of these hints is to help those who use the book to study graph theory on their own,

but  not  to spoil the fun. The exercises, including hints, continue to be intended for classroom use.

Apart from these two changes, there are a few additions. The most

noticable of these are the formal introduction of depth-first search trees in Section 1.5 (which has led to some simplifications in later proofs) and an ingenious new proof of Menger’s theorem due to Böhme, Göring and

Harant (which has not otherwise been published).

Finally, there is a host of small simplifications and clarifications

of arguments that I noticed as I taught from the book, or which were

pointed out to me by others. To all these I offer my special thanks.

The Web site for the book has followed me to

http://www.math.uni-hamburg.de/home/diestel/books/graph.theory/

I expect this address to be stable for some time.

Once more, my thanks go to all who contributed to this second

edition by commenting on the first—and I look forward to further com-

ments!

December 1999

RD

Contents

Preface  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  vii

1. The Basics  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

1

1.1. Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

2

1.2. The degree of a vertex  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

4

1.3. Paths and cycles  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

6

1.4. Connectivity  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

9

1.5. Trees and forests  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

12

1.6. Bipartite graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

14

1.7. Contraction and minors  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

16

1.8. Euler tours  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

18

1.9. Some linear algebra  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

20

1.10. Other notions of graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

25

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

26

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

28

2. Matching  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  29

2.1. Matching in bipartite graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

29

2.2. Matching in general graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

34

2.3. Path covers  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

39

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

40

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

42

xii

Contents

3. Connectivity  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

43

3.1. 2-Connected graphs and subgraphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

43

3.2. The structure of 3-connected graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . .

45

3.3. Menger’s theorem  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

50

3.4. Mader’s theorem  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

56

3.5. Edge-disjoint spanning trees  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

58

3.6. Paths between given pairs of vertices  . . . . . . . . . . . . . . . . . . . . . . . . . . .

61

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

63

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

65

4. Planar Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  67

4.1. Topological prerequisites  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

68

4.2. Plane graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

70

4.3. Drawings  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

76

4.4. Planar graphs: Kuratowski’s theorem  . . . . . . . . . . . . . . . . . . . . . . . . . . .

80

4.5. Algebraic planarity criteria  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

85

4.6. Plane duality  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

87

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

89

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

92

5. Colouring  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  95

5.1. Colouring maps and planar graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

96

5.2. Colouring vertices  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

98

5.3. Colouring edges  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  103

5.4. List colouring  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  105

5.5. Perfect graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  110

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  117

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  120

6. Flows  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  123

6.1. Circulations  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  124

6.2. Flows in networks  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  125

6.3. Group-valued flows  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  128

6.4.  k-Flows for small  k  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  133

6.5. Flow-colouring duality  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  136

6.6. Tutte’s flow conjectures  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  140

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  144

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  145

Contents

xiii

7. Substructures in Dense Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  147

7.1. Subgraphs  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  148

7.2. Szemer´

edi’s regularity lemma  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  153

7.3. Applying the regularity lemma  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  160

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  165

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  166

8. Substructures in Sparse Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  169

8.1. Topological minors  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  170

8.2. Minors  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  179

8.3. Hadwiger’s conjecture  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  181

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  184

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  186

9. Ramsey Theory for Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  189

9.1. Ramsey’s original theorems  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  190

9.2. Ramsey numbers  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  193

9.3. Induced Ramsey theorems  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  197

9.4. Ramsey properties and connectivity  . . . . . . . . . . . . . . . . . . . . . . . . . . . .  207

Exercises  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  208

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  210

10. Hamilton Cycles  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  213

10.1. Simple sufficient conditions  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  213

10.2. Hamilton cycles and degree sequences  . . . . . . . . . . . . . . . . . . . . . . . . . . .  216

10.3. Hamilton cycles in the square of a graph  . . . . . . . . . . . . . . . . . . . . . . . .  218

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  226

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  227

11. Random Graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  229

11.1. The notion of a random graph  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  230

11.2. The probabilistic method  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  235

11.3. Properties of almost all graphs  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  238

11.4. Threshold functions and second moments  . . . . . . . . . . . . . . . . . . . . . . .  242

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  247

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  249

xiv

Contents

12. Minors, Trees, and WQO  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  251

12.1. Well-quasi-ordering  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  251

12.2. The graph minor theorem for trees  . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  253

12.3. Tree-decompositions  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  255

12.4. Tree-width and forbidden minors  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  263

12.5. The graph minor theorem  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  274

Exercises  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  277

Notes  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  280

Hints for all the exercises  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  283

Index  .  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  299

Symbol index  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .  311

1

The Basics

This chapter gives a gentle yet concise introduction to most of the terminology used later in the book. Fortunately, much of standard graph

theoretic terminology is so intuitive that it is easy to remember; the few terms better understood in their proper setting will be introduced later, when their time has come.

Section 1.1 offers a brief but self-contained summary of the most

basic definitions in graph theory, those centred round the notion of a

graph. Most readers will have met these definitions before, or will have them explained to them as they begin to read this book. For this reason, Section 1.1 does not dwell on these definitions more than clarity requires: its main purpose is to collect the most basic terms in one place, for easy reference later.

From Section 1.2 onwards, all new definitions will be brought to life

almost immediately by a number of simple yet fundamental propositions.

Often, these will relate the newly defined terms to one another: the

question of how the value of one invariant influences that of another

underlies much of graph theory, and it will be good to become familiar

with this line of thinking early.

By N we denote the set of natural numbers, including zero. The set

Z /n Z of integers modulo  n  is denoted by Z n; its elements are written as Z n

i :=  i +  n Z. For a real number  x  we denote by  bxc  the greatest integer 6  x, and by  dxe  the least integer >  x. Logarithms written as ‘log’ are bxc, dxe

taken at base 2; the natural logarithm will be denoted by ‘ln’. A set

log ,  ln

A =  { A 1 , . . . , Ak }  of disjoint subsets of a set  A  is a  partition  of  A  if partition

S

A =

k

A

}  of

i=1

i  and  Ai 6=  ∅  for every  i. Another partition  { A0 1 , . . . , A0`

A refines  the partition  A  if each  A0  is contained in some  A i

j . By [ A] k  we

[ A] k

denote the set of all  k-element subsets of  A. Sets with  k  elements will be called  k-sets; subsets with  k  elements are  k-subsets.

k-set

2

1. The Basics

1.1 Graphs

graph

A  graph  is a pair  G = ( V, E) of sets satisfying  E ⊆ [ V ]2; thus, the elements of  E  are 2-element subsets of  V . To avoid notational ambiguities, we shall always assume tacitly that  V ∩ E =  ∅. The elements of  V  are the vertex

vertices (or  nodes, or  points) of the graph  G, the elements of  E  are its edge

edges (or  lines). The usual way to picture a graph is by drawing a dot for each vertex and joining two of these dots by a line if the corresponding two vertices form an edge. Just how these dots and lines are drawn is

considered irrelevant: all that matters is the information which pairs of vertices form an edge and which do not.

3

7

5

1

6

2

4

Fig. 1.1.1.  The graph on  V =  {  1 , . . . ,  7  }  with edge set E =  {{  1 ,  2  }, {  1 ,  5  }, {  2 ,  5  }, {  3 ,  4  }, {  5 ,  7  }}

on

A graph with vertex set  V  is said to be a graph  on V . The vertex V ( G) , E( G) set of a graph  G  is referred to as  V ( G), its edge set as  E( G). These conventions are independent of any actual names of these two sets: the

vertex set  W  of a graph  H = ( W, F ) is still referred to as  V ( H), not as W ( H). We shall not always distinguish strictly between a graph and its vertex or edge set. For example, we may speak of a vertex  v ∈ G (rather than  v ∈ V ( G)), an edge  e ∈ G, and so on.

order

The number of vertices of a graph  G  is its  order , written as  |G|;

|G|, kGk

its number of edges is denoted by  kGk. Graphs are  finite  or  infinite according to their order; unless otherwise stated, the graphs we consider are all finite.

∅

For the  empty graph ( ∅, ∅) we simply write  ∅. A graph of order 0 or 1

trivial

is called  trivial . Sometimes, e.g. to start an induction, trivial graphs can graph

be useful; at other times they form silly counterexamples and become a

nuisance. To avoid cluttering the text with non-triviality conditions, we shall mostly treat the trivial graphs, and particularly the empty graph  ∅, with generous disregard.

incident

A vertex  v  is  incident  with an edge  e  if  v ∈ e; then  e  is an edge  at v.

ends

The two vertices incident with an edge are its  endvertices  or  ends, and an edge  joins  its ends. An edge  { x, y }  is usually written as  xy (or  yx).

If  x ∈ X  and  y ∈ Y , then  xy  is an  X– Y edge. The set of all  X– Y  edges E( X, Y )

in a set  E  is denoted by  E( X, Y ); instead of  E( { x }, Y ) and  E( X, { y }) we simply write  E( x, Y ) and  E( X, y). The set of all the edges in  E  at a E( v)

vertex  v  is denoted by  E( v).

1.1 Graphs

3

Two vertices  x, y  of  G  are  adjacent , or  neighbours, if  xy  is an edge adjacent

of  G. Two edges  e 6=  f  are  adjacent  if they have an end in common. If all neighbour

the vertices of  G  are pairwise adjacent, then  G  is  complete. A complete complete

graph on  n  vertices is a  Kn; a  K 3 is called a  triangle.

Kn

Pairwise non-adjacent vertices or edges are called  independent .

More formally, a set of vertices or of edges is  independent (or  stable) independent

if no two of its elements are adjacent.

Let  G = ( V, E) and  G0 = ( V 0, E0) be two graphs. We call  G  and G0 isomorphic, and write  G ' G0, if there exists a bijection  ϕ:  V → V 0

'

with  xy ∈ E ⇔ ϕ( x) ϕ( y)  ∈ E0  for all  x, y ∈ V . Such a map  ϕ  is called isomor-an  isomorphism; if  G =  G0, it is called an  automorphism. We do not phism

normally distinguish between isomorphic graphs. Thus, we usually write

G =  G0  rather than  G ' G0, speak of  the  complete graph on 17 vertices, and so on. A map taking graphs as arguments is called a  graph invariant invariant

if it assigns equal values to isomorphic graphs. The number of vertices and the number of edges of a graph are two simple graph invariants; the greatest number of pairwise adjacent vertices is another.

1

4

2

4

6

3

5

3

5

G

G0

1

1

4

2

4

2

6

3

5

3

5

G ∪

G − G0

G0

G ∩ G0

Fig. 1.1.2.  Union, difference and intersection; the vertices 2,3,4

induce (or span) a triangle in  G ∪ G0  but not in  G

We set  G ∪ G0 := ( V ∪ V 0, E ∪ E0) and  G ∩ G0 := ( V ∩ V 0, E ∩ E0).

G ∩ G0

If  G ∩ G0 =  ∅, then  G  and  G0  are  disjoint. If  V 0 ⊆ V  and  E0 ⊆ E, then subgraph

G0  is a  subgraph  of  G (and  G  a  supergraph  of  G0), written as  G0 ⊆ G.

G0 ⊆ G

Less formally, we say that  G contains G0.

If  G0 ⊆ G  and  G0  contains  all  the edges  xy ∈ E  with  x, y ∈ V 0, then G0  is an  induced subgraph  of  G; we say that  V 0 induces  or  spans G0  in  G, induced

subgraph

and write  G0 =:  G [  V 0 ]. Thus if  U ⊆ V  is any set of vertices, then  G [  U ]

G [  U ]

denotes the graph on  U  whose edges are precisely the edges of  G  with both ends in  U . If  H  is a subgraph of  G, not necessarily induced, we abbreviate  G [  V ( H) ] to  G [  H ]. Finally,  G0 ⊆ G  is a  spanning  subgraph spanning

of  G  if  V 0  spans all of  G, i.e. if  V 0 =  V .

4

1. The Basics

G0

G00

G

Fig. 1.1.3.  A graph  G  with subgraphs  G0  and  G00: G0  is an induced subgraph of  G, but  G00  is not

−

If  U  is any set of vertices (usually of  G), we write  G − U  for G [  V  r  U ]. In other words,  G − U  is obtained from  G  by  deleting  all the vertices in  U ∩ V  and their incident edges. If  U =  { v }  is a singleton, we write  G − v  rather than  G − { v }. Instead of  G − V ( G0) we simply

+

write  G − G0. For a subset  F  of [ V ]2 we write  G − F := ( V, E  r  F ) and G +  F := ( V, E ∪ F ); as above,  G − { e }  and  G +  { e }  are abbreviated to edge-G − e  and  G +  e. We call  G edge-maximal  with a given graph property maximal

if  G  itself has the property but no graph  G +  xy  does, for non-adjacent vertices  x, y ∈ G.

minimal

More generally, when we call a graph  minimal  or  maximal  with some maximal

property but have not specified any particular ordering, we are referring to the subgraph relation. When we speak of minimal or maximal sets of

vertices or edges, the reference is simply to set inclusion.

G ∗ G0

If  G  and  G0  are disjoint, we denote by  G ∗ G0  the graph obtained from  G ∪ G0  by joining all the vertices of  G  to all the vertices of  G0. For comple-example,  K 2  ∗ K 3 =  K 5. The  complement G  of  G  is the graph on  V

ment G

with edge set [ V ]2 r  E. The  line graph L( G) of  G  is the graph on  E  in line graph

which  x, y ∈ E  are adjacent as vertices if and only if they are adjacent L( G)

as edges in  G.

G

G

Fig. 1.1.4.  A graph isomorphic to its complement

1.2 The degree of a vertex

Let  G = ( V, E) be a (non-empty) graph. The set of neighbours of a N ( v)

vertex  v  in  G  is denoted by  NG( v), or briefly by  N ( v).1 More generally 1 Here, as elsewhere, we drop the index referring to the underlying graph if the reference is clear.

1.2 The degree of a vertex

5

for  U ⊆ V , the neighbours in  V  r  U  of vertices in  U  are called  neighbours of U ; their set is denoted by  N ( U ).

The  degree (or  valency)  dG( v) =  d( v) of a vertex  v  is the number  degree d( v)

|E( v) |  of edges at  v; by our definition of a graph,2 this is equal to the number of neighbours of  v. A vertex of degree 0 is  isolated . The number isolated

δ( G) := min  { d( v)  | v ∈ V }  is the  minimum degree  of  G, the number δ( G)

∆( G) := max  { d( v)  | v ∈ V }  its  maximum degree. If all the vertices

∆( G)

of  G  have the same degree  k, then  G  is  k- regular , or simply  regular . A regular

3-regular graph is called  cubic.

cubic

The number

1 X

d( G) :=  |

d( v)

V | v∈V

d( G)

average

is the  average degree  of  G. Clearly,

degree

δ( G) 6  d( G) 6 ∆( G)  .

The average degree quantifies globally what is measured locally by the

vertex degrees: the number of edges of  G  per vertex. Sometimes it will be convenient to express this ratio directly, as  ε( G) :=  |E|/|V |.

ε( G)

The quantities  d  and  ε  are, of course, intimately related. Indeed, if we sum up all the vertex degrees in  G, we count every edge exactly twice: once from each of its ends. Thus

X

|E| = 1

d( v) = 1  d( G)  · |V | ,

2

2

v ∈V

and therefore

ε( G) = 1  d( G)  .

2

Proposition 1.2.1.  The number of vertices of odd degree in a graph is

[ 10.3.3 ]

always even.

P

P

Proof . A graph on  V  has 1

d( v) edges, so

d( v) is an even

2

v ∈V

number.

¤

If a graph has large minimum degree, i.e. everywhere, locally, many

edges per vertex, it also has many edges per vertex globally:  ε( G) =

1  d( G) > 1 δ( G). Conversely, of course, its average degree may be large 2

2

even when its minimum degree is small. However, the vertices of large

degree cannot be scattered completely among vertices of small degree: as the next proposition shows, every graph  G  has a subgraph whose average degree is no less than the average degree of  G, and whose minimum degree is more than half its average degree:

2 but not for multigraphs; see Section 1.10

6

1. The Basics

[ 3.6.1 ]

Proposition 1.2.2.  Every graph G with at least one edge has a subgraph H with δ( H)  > ε( H) >  ε( G) .

Proof . To construct  H  from  G, let us try to delete vertices of small degree one by one, until only vertices of large degree remain. Up to

which degree  d( v) can we afford to delete a vertex  v, without lowering  ε?

Clearly, up to  d( v) =  ε : then the number of vertices decreases by 1

and the number of edges by at most  ε, so the overall ratio  ε  of edges to vertices will not decrease.

Formally, we construct a sequence  G =  G 0  ⊇ G 1  ⊇ . . .  of induced subgraphs of  G  as follows. If  Gi  has a vertex  vi  of degree  d( vi) 6  ε( Gi), we let  Gi+1 :=  Gi − vi; if not, we terminate our sequence and set H :=  Gi. By the choices of  vi  we have  ε( Gi+1) >  ε( Gi) for all  i, and hence  ε( H) >  ε( G).

What else can we say about the graph  H? Since  ε( K 1) = 0  < ε( G), none of the graphs in our sequence is trivial, so in particular  H 6=  ∅. The fact that  H  has no vertex suitable for deletion thus implies  δ( H)  > ε( H), as claimed.

¤

1.3 Paths and cycles

path

A  path  is a non-empty graph  P = ( V, E) of the form V =  { x 0 , x 1 , . . . , xk }

E =  { x 0 x 1 , x 1 x 2 , . . . , xk− 1 xk } , where the  xi  are all distinct. The vertices  x 0 and  xk  are  linked  by  P  and are called its  ends; the vertices  x 1 , . . . , xk− 1 are the  inner  vertices of  P .

length

The number of edges of a path is its  length, and the path of length  k  is P k

denoted by  P k. Note that  k  is allowed to be zero; thus,  P  0 =  K 1.

G

P

Fig. 1.3.1.  A path  P =  P  6 in  G

We often refer to a path by the natural sequence of its vertices,3

writing, say,  P =  x 0 x 1  . . . xk  and calling  P  a path  from x 0  to xk (as well as  between x 0 and  xk).

3 More precisely, by one of the two natural sequences:  x 0  . . . xk  and  xk . . . x 0

denote the same path. Still, it often helps to fix one of these two orderings of  V ( P ) notationally: we may then speak of things like the ‘first’ vertex on  P  with a certain property, etc.

1.3 Paths and cycles

7

For 0 6  i  6  j  6  k  we write

xP y, ˚

P

P xi :=  x 0  . . . xi

xiP :=  xi . . . xk

xiP xj :=  xi . . . xj

and

˚

P :=  x 1  . . . xk− 1

P˚

xi :=  x 0  . . . xi− 1

˚

xiP :=  xi+1  . . . xk

˚

xiP˚

xj :=  xi+1  . . . xj− 1

for the appropriate subpaths of  P . We use similar intuitive notation for the concatenation of paths; for example, if the union  P x ∪ xQy ∪ yR  of three paths is again a path, we may simply denote it by  P xQyR.

P xQyR

P

y

y

z

z

x

x

xP yQz

Q

Fig. 1.3.2.  Paths  P ,  Q  and  xP yQz

Given sets  A, B  of vertices, we call  P =  x 0  . . . xk  an  A–B path  if A–B path

V ( P )  ∩ A =  { x 0  }  and  V ( P )  ∩ B =  { xk }. As before, we write  a– B

path rather than  { a }– B  path, etc. Two or more paths are  independent independent

if none of them contains an inner vertex of another. Two  a– b  paths, for instance, are independent if and only if  a  and  b  are their only common vertices.

Given a graph  H, we call  P  an  H-path  if  P  is non-trivial and meets H-path

H  exactly in its ends. In particular, the edge of any  H-path of length 1

is never an edge of  H.

If  P =  x 0  . . . xk− 1 is a path and  k > 3, then the graph  C :=

P +  xk− 1 x 0 is called a  cycle. As with paths, we often denote a cycle cycle

by its (cyclic) sequence of vertices; the above cycle  C  might be written as  x 0  . . . xk− 1 x 0. The  length  of a cycle is its number of edges (or vertices); length

the cycle of length  k  is called a  k-cycle  and denoted by  Ck.

Ck

The minimum length of a cycle (contained) in a graph  G  is the  girth girth g( G)

g( G) of  G; the maximum length of a cycle in  G  is its  circumference. (If circum-G  does not contain a cycle, we set the former to  ∞, the latter to zero.) ference

An edge which joins two vertices of a cycle but is not itself an edge of chord

the cycle is a  chord  of that cycle. Thus, an  induced cycle  in  G, a cycle in G  forming an induced subgraph, is one that has no chords (Fig. 1.3.3).

induced

cycle

8

1. The Basics

x

y

Fig. 1.3.3.  A cycle  C 8 with chord  xy, and induced cycles  C 6 , C 4

If a graph has large minimum degree, it contains long paths and

cycles:

[ 3.6.1 ]

Proposition 1.3.1.  Every graph G contains a path of length δ( G)  and a cycle of length at least δ( G) + 1  (provided that δ( G) > 2 ).

Proof . Let  x 0  . . . xk  be a longest path in  G. Then all the neighbours of xk  lie on this path (Fig. 1.3.4). Hence  k >  d( xk) >  δ( G). If  i < k  is minimal with  xixk ∈ E( G), then  xi . . . xkxi  is a cycle of length at least δ( G) + 1.

¤

x 0

xi

xk

Fig. 1.3.4.  A longest path  x 0  . . . xk, and the neighbours of  xk Minimum degree and girth, on the other hand, are not related (unless we fix the number of vertices): as we shall see in Chapter 11, there are graphs combining arbitrarily large minimum degree with arbitrarily

large girth.

distance

The  distance dG( x, y) in  G  of two vertices  x, y  is the length of a dG( x, y)

shortest  x– y  path in  G; if no such path exists, we set  d( x, y) :=  ∞. The greatest distance between any two vertices in  G  is the  diameter  of  G, diameter

denoted by diam( G). Diameter and girth are, of course, related: diam( G)

Proposition 1.3.2.  Every graph G containing a cycle satisfies g( G) 6

2 diam( G) + 1 .

Proof . Let  C  be a shortest cycle in  G. If  g( G) > 2 diam( G) + 2, then C  has two vertices whose distance in  C  is at least diam( G) + 1. In  G, these vertices have a lesser distance; any shortest path  P  between them is therefore not a subgraph of  C. Thus,  P  contains a  C-path  xP y.

Together with the shorter of the two  x– y  paths in  C, this path  xP y forms a shorter cycle than  C, a contradiction.

¤

1.3 Paths and cycles 9

A vertex is  central  in  G  if its greatest distance from any other ver-  central tex is as small as possible. This distance is the  radius  of  G, denoted  radius by rad( G). Thus, formally, rad( G) = min x∈V ( G) max y∈V ( G)  dG( x, y). rad( G) As one easily checks (exercise), we have

rad( G) 6 diam( G) 6 2 rad( G)  .

Diameter and radius are not directly related to the minimum or

average degree: a graph can combine large minimum degree with large

diameter, or small average degree with small diameter (examples?).

The maximum degree behaves differently here: a graph of large

order can only have small radius and diameter if its maximum degree

is large. This connection is quantified very roughly in the following

proposition:

Proposition 1.3.3.  A graph G of radius at most k and maximum degree

[ 9.4.1 ]

[ 9.4.2 ]

at most d has no more than  1 +  kdk vertices.

Proof . Let  z  be a central vertex in  G, and let  Di  denote the set of S

vertices of  G  at distance  i  from  z. Then  V ( G) =

k

D

i=0

i, and  |D 0 | = 1.

Since ∆( G) 6  d, we have  |Di|  6  d |Di− 1 |  for  i = 1 , . . . , k, and thus

|Di|  6  di  by induction. Adding up these inequalities we obtain k

X

|G|  6 1 +

di  6 1 +  kdk.

i=1

¤

A  walk (of  length k) in a graph  G  is a non-empty alternating se-  walk quence  v 0 e 0 v 1 e 1  . . . ek− 1 vk  of vertices and edges in  G  such that  ei =

{ vi, vi+1  }  for all  i < k. If  v 0 =  vk, the walk is  closed. If the vertices in a walk are all distinct, it defines an obvious path in  G. In general, every walk between two vertices contains4 a path between these vertices (proof?).

1.4 Connectivity

A non-empty graph  G  is called  connected  if any two of its vertices are  connected linked by a path in  G. If  U ⊆ V ( G) and  G [  U ] is connected, we also call U  itself connected (in  G).

Proposition 1.4.1.  The vertices of a connected graph G can always be

[ 1.5.2 ]

enumerated, say as v 1 , . . . , vn, so that Gi :=  G [  v 1 , . . . , vi ]  is connected for every i.

4 We shall often use terms defined for graphs also for walks, as long as their meaning is obvious.

10

1. The Basics

Proof . Pick any vertex as  v 1, and assume inductively that  v 1 , . . . , vi have been chosen for some  i < |G|. Now pick a vertex  v ∈ G − Gi. As  G

is connected, it contains a  v– v 1 path  P . Choose as  vi+1 the last vertex of  P  in  G − Gi; then  vi+1 has a neighbour in  Gi. The connectedness of every  Gi  follows by induction on  i.

¤

Let  G = ( V, E) be a graph. A maximal connected subgraph of  G

component

is called a  component  of  G. Note that a component, being connected, is always non-empty; the empty graph, therefore, has no components.

Fig. 1.4.1.  A graph with three components, and a minimal

spanning connected subgraph in each component

If  A, B ⊆ V  and  X ⊆ V ∪ E  are such that every  A– B  path in separate

G  contains a vertex or an edge from  X, we say that  X separates  the sets  A  and  B  in  G. This implies in particular that  A ∩ B ⊆ X. More generally we say that  X separates G, and call  X  a  separating set  in  G, if  X  separates two vertices of  G − X  in  G. A vertex which separates cutvertex

two other vertices of the same component is a  cutvertex , and an edge bridge

separating its ends is a  bridge. Thus, the bridges in a graph are precisely those edges that do not lie on any cycle.

v

x

y

w

e

Fig. 1.4.2.  A graph with cutvertices  v, x, y, w  and bridge  e =  xy k-connected

G  is called  k-connected (for  k ∈  N) if  |G| > k  and  G − X  is connected for every set  X ⊆ V  with  |X| < k. In other words, no two vertices of  G

are separated by fewer than  k  other vertices. Every (non-empty) graph is 0-connected, and the 1-connected graphs are precisely the non-trivial connected graphs. The greatest integer  k  such that  G  is  k-connected connectivity  is the  connectivity κ( G) of  G. Thus,  κ( G) = 0 if and only if  G  is κ( G)

disconnected or a  K 1, and  κ( Kn) =  n −  1 for all  n > 1.

If  |G| >  1 and  G − F  is connected for every set  F ⊆ E  of fewer

`-edge-

than  `  edges, then  G  is called  `-edge-connected. The greatest integer  `

connected

1.4 Connectivity

11

G

H

Fig. 1.4.3.  The  octahedron G (left) with  κ( G) =  λ( G) = 4, and a graph  H  with  κ( H) = 2 but  λ( H) = 4

such that  G  is  `-edge-connected is the  edge-connectivity λ( G) of  G. In edge-particular, we have  λ( G) = 0 if  G  is disconnected.

connectivity

λ( G)

For every non-trivial graph  G  we have

κ( G) 6  λ( G) 6  δ( G)

(exercise), so in particular high connectivity requires a large minimum degree. Conversely, large minimum degree does not ensure high connectivity, not even high edge-connectivity (examples?). It does, however,

imply the existence of a highly connected subgraph:

Theorem 1.4.2. (Mader 1972)

Every graph of average degree at least  4 k has a k-connected subgraph.

[ 8.1.1 ]

[ 11.2.3 ]

Proof . For  k ∈ {  0 ,  1  }  the assertion is trivial; we consider  k > 2 and a graph  G = ( V, E) with  |V | =:  n  and  |E| =:  m. For inductive reasons it will be easier to prove the stronger assertion that  G  has a  k-connected subgraph whenever

(i)  n > 2 k −  1 and

(ii)  m > (2 k −  3)( n − k + 1) + 1.

(This assertion is indeed stronger, i.e. (i) and (ii) follow from our assumption of  d( G) > 4 k: (i) holds since  n > ∆( G) >  d( G) > 4 k, while (ii) follows from  m = 1  d( G) n > 2 kn.) 2

We apply induction on  n. If  n = 2 k −  1, then  k = 1 ( n + 1), and 2

hence  m > 1  n( n −  1) by (ii). Thus  G =  Kn ⊇ Kk+1, proving our claim.

2

We now assume that  n > 2 k. If  v  is a vertex with  d( v) 6 2 k −  3, we can apply the induction hypothesis to  G − v  and are done. So we assume that δ( G) > 2 k −  2. If  G  is  k-connected, there is nothing to show. We may therefore assume that  G  has the form  G =  G 1  ∪ G 2 with  |G 1  ∩ G 2 | < k and  |G 1 |, |G 2 | < n. As every edge of  G  lies in  G 1 or in  G 2,  G  has no edge between  G 1  − G 2 and  G 2  − G 1. Since each vertex in these subgraphs has at least  δ( G) > 2 k −  2 neighbours, we have  |G 1 |, |G 2 | > 2 k −  1. But then at least one of the graphs  G 1 , G 2 must satisfy the induction hypothesis

12

1. The Basics

(completing the proof): if neither does, we have

kGik  6 (2 k −  3)( |Gi| − k + 1)

for  i = 1 ,  2, and hence

m  6  kG 1 k +  kG 2 k

¡

¢

6 (2 k −  3)  |G 1 | +  |G 2 | −  2 k + 2

6 (2 k −  3)( n − k + 1)

(by  |G 1  ∩ G 2 |  6  k −  1)

contradicting (ii).

¤

1.5 Trees and forests

forest

An  acyclic  graph, one not containing any cycles, is called a  forest. A con-tree

nected forest is called a  tree. (Thus, a forest is a graph whose components leaf

are trees.) The vertices of degree 1 in a tree are its  leaves. Every nontrivial tree has at least two leaves—take, for example, the ends of a

longest path. This little fact often comes in handy, especially in induction proofs about trees: if we remove a leaf from a tree, what remains is still a tree.

Fig. 1.5.1.  A tree

[ 1.6.1 ]

[ 1.9.6 ]

Theorem 1.5.1.  The following assertions are equivalent for a graph T :

[ 4.2.7 ]

(i)  T is a tree;

(ii)  any two vertices of T are linked by a unique path in T ;

(iii)  T is minimally connected, i.e. T is connected but T − e is disconnected for every edge e ∈ T ;

(iv)  T is maximally acyclic, i.e. T contains no cycle but T +  xy does, for any two non-adjacent vertices x, y ∈ T .

¤

1.5 Trees and forests

13

The proof of Theorem 1.5.1 is straightforward, and a good exercise

for anyone not yet familiar with all the notions it relates. Extending our notation for paths from Section 1.3, we write  xT y  for the unique path xT y

in a tree  T  between two vertices  x, y (see (ii) above).

A frequently used application of Theorem 1.5.1 is that every con-

nected graph contains a spanning tree: by the equivalence of (i) and (iii), any minimal connected spanning subgraph will be a tree. Figure 1.4.1

shows a spanning tree in each of the three components of the graph

depicted.

Corollary 1.5.2.  The vertices of a tree can always be enumerated, say as v 1 , . . . , vn, so that every vi with i > 2  has a unique neighbour in

{ v 1 , . . . , vi− 1  }.

Proof . Use the enumeration from Proposition 1.4.1.

¤

(1.4.1)

[ 1.9.6 ]

Corollary 1.5.3.  A connected graph with n vertices is a tree if and

[ 3.5.1 ]

[ 3.5.4 ]

only if it has n −  1  edges.

[ 4.2.7 ]

[ 8.2.2 ]

Proof . Induction on  i  shows that the subgraph spanned by the first i  vertices in Corollary 1.5.2 has  i −  1 edges; for  i =  n  this proves the forward implication. Conversely, let  G  be any connected graph with  n vertices and  n −  1 edges. Let  G0  be a spanning tree in  G. Since  G0  has n −  1 edges by the first implication, it follows that  G =  G0.

¤

Corollary 1.5.4.  If T is a tree and G is any graph with δ( G) >  |T | −  1 ,

[ 9.2.1 ]

[ 9.2.3 ]

then T ⊆ G, i.e. G has a subgraph isomorphic to T .

Proof . Find a copy of  T  in  G  inductively along its vertex enumeration from Corollary 1.5.2.

¤

Sometimes it is convenient to consider one vertex of a tree as special; such a vertex is then called the  root  of this tree. A tree with a fixed root root

is a  rooted tree. Choosing a root  r  in a tree  T  imposes a partial ordering on  V ( T ) by letting  x  6  y  if  x ∈ rT y. This is the  tree-order  on  V ( T ) tree-order

associated with  T  and  r. Note that  r  is the least element in this partial order, every leaf  x 6=  r  of  T  is a maximal element, the ends of any edge of  T  are comparable, and every set of the form  { x | x  6  y } (where  y is any fixed vertex) is a  chain, a set of pairwise comparable elements.

chain

(Proofs?)

A rooted tree  T  contained in a graph  G  is called  normal  in  G  if  normal tree the ends of every  T -path in  G  are comparable in the tree-order of  T .

If  T  spans  G, this amounts to requiring that two vertices of  T  must be comparable whenever they are adjacent in  G; see Figure 1.5.2. Normal spanning trees are also called  depth-first search trees, because of the way they arise in computer searches on graphs (Exercise 17).

14

1. The Basics

G

T

r

Fig. 1.5.2.  A depth-first search tree with root  r

Normal spanning trees provide a simple but powerful structural tool

in graph theory. And they always exist:

[ 6.5.3 ]

Proposition 1.5.5.  Every connected graph contains a normal spanning tree, with any specified vertex as its root.

Proof . Let  G  be a connected graph and  r ∈ G  any specified vertex. Let  T

be a maximal normal tree with root  r  in  G; we show that  V ( T ) =  V ( G).

Suppose not, and let  C  be a component of  G − T . As  T  is normal, N ( C) is a chain in  T . Let  x  be its greatest element, and let  y ∈ C  be adjacent to  x. Let  T 0  be the tree obtained from  T  by joining  y  to  x; the tree-order of  T 0  then extends that of  T . We shall derive a contradiction by showing that  T 0  is also normal in  G.

Let  P  be a  T 0-path in  G. If the ends of  P  both lie in  T , then they are comparable in the tree-order of  T (and hence in that of  T 0), because then  P  is also a  T -path and  T  is normal in  G  by assumption. If not, then  y  is one end of  P , so  P  lies in  C  except for its other end  z, which lies in  N ( C). Then  z  6  x, by the choice of  x. For our proof that  y  and z  are comparable it thus suffices to show that  x < y, i.e. that  x ∈ rT 0y.

This, however, is clear since  y  is a leaf of  T 0  with neighbour  x.

¤

1.6 Bipartite graphs

r-partite

Let  r > 2 be an integer. A graph  G = ( V, E) is called  r-partite  if V  admits a partition into  r  classes such that every edge has its ends in different classes: vertices in the same partition class must not be

bipartite

adjacent. Instead of ‘2-partite’ one usually says  bipartite.

An  r-partite graph in which  every  two vertices from different par-complete

tition classes are adjacent is called  complete; the complete  r-partite r-partite

graphs for all  r  together are the  complete multipartite  graphs.

The

1.6 Bipartite graphs 15

K 2 ,  2 ,  2 =  K 32

Fig. 1.6.1.  Two 3-partite graphs

complete  r-partite graph  Kn 1  ∗ . . . ∗ Knr  is denoted by  Kn

; if

K

1  ,...,nr

n 1 ,...,nr

n 1 =  . . . =  nr =:  s, we abbreviate this to  Krs. Thus,  Krs  is the complete  Krs r-partite graph in which every partition class contains exactly  s  vertices.5 (Figure 1.6.1 shows the example of the octahedron  K 32; compare its drawing with that in Figure 1.4.3.) Graphs of the form  K 1 ,n  are called  stars.  star

=

=

Fig. 1.6.2.  Three drawings of the bipartite graph  K 3 ,  3 =  K 23

Clearly, a bipartite graph cannot contain an  odd cycle, a cycle of odd  odd cycle length. In fact, the bipartite graphs are characterized by this property: Proposition 1.6.1.  A graph is bipartite if and only if it contains no

[ 5.3.1 ]

[ 6.4.2 ]

odd cycle.

Proof . Let  G = ( V, E) be a graph without odd cycles; we show that  G  is

(1.5.1)

bipartite. Clearly a graph is bipartite if all its components are bipartite or trivial, so we may assume that  G  is connected. Let  T  be a spanning tree in  G, pick a root  r ∈ T , and denote the associated tree-order on  V

by 6 T . For each  v ∈ V , the unique path  rT v  has odd or even length.

This defines a bipartition of  V ; we show that  G  is bipartite with this partition.

Let  e =  xy  be an edge of  G. If  e ∈ T , with  x <T y  say, then rT y =  rT xy  and so  x  and  y  lie in different partition classes. If  e /

∈ T

then  Ce :=  xT y +  e  is a cycle (Fig. 1.6.3), and by the case treated already the vertices along  xT y  alternate between the two classes. Since Ce  is even by assumption,  x  and  y  again lie in different classes. ¤

5 Note that we obtain a  Krs  if we replace each vertex of a  Kr  by an independent s-set; our notation of  Kr

s  is intended to hint at this connection.

16

1. The Basics

x

e

Ce

y

r

Fig. 1.6.3.  The cycle  Ce  in  T +  e

1.7 Contraction and minors

In Section 1.1 we saw two fundamental containment relations between

graphs: the subgraph relation, and the ‘induced subgraph’ relation. In

this section we meet another: the minor relation.

G/e

Let  e =  xy  be an edge of a graph  G = ( V, E). By  G/e  we denote the contraction

graph obtained from  G  by  contracting  the edge  e  into a new vertex  ve, which becomes adjacent to all the former neighbours of  x  and of  y. Formally,  G/e  is a graph ( V 0, E0) with vertex set  V 0 := ( V  r  { x, y })  ∪ { ve }

ve

(where  ve  is the ‘new’ vertex, i.e.  ve /

∈ V ∪ E) and edge set

n

o

E0 :=

vw ∈ E | { v, w } ∩ { x, y } =  ∅

n

o

∪ vew | xw ∈ E  r  { e }  or  yw ∈ E  r  { e } .

x

ve

e

G

G/e

y

Fig. 1.7.1.  Contracting the edge  e =  xy

More generally, if  X  is another graph and  { Vx | x ∈ V ( X)  }  is a partition of  V  into connected subsets such that, for any two vertices x, y ∈ X, there is a  Vx– Vy  edge in  G  if and only if  xy ∈ E( X), we call M X

G  an  M X  and write6  G =  M X (Fig. 1.7.2). The sets  Vx  are the  branch branch sets

sets  of this  M X. Intuitively, we obtain  X  from  G  by contracting every 6 Thus formally, the expression  MX—where  M  stands for ‘minor’; see below—

refers to a whole class of graphs, and  G =  M X  means (with slight abuse of notation) that  G  belongs to this class.

1.7 Contraction and minors

17

Vz

z

x

X

V

Y

x

G

Fig. 1.7.2. Y ⊇ G =  M X, so  X  is a minor of  Y

branch set to a single vertex and deleting any ‘parallel edges’ or ‘loops’

that may arise.

If  Vx =  U ⊆ V  is one of the branch sets above and every other branch set consists just of a single vertex, we also write  G/U  for the G/U

graph  X  and  vU  for the vertex  x ∈ X  to which  U  contracts, and think vU

of the rest of  X  as an induced subgraph of  G. The contraction of a single edge  uu0  defined earlier can then be viewed as the special case of U =  { u, u0 }.

Proposition 1.7.1.  G is an M X if and only if X can be obtained from G by a series of edge contractions, i.e. if and only if there are graphs G 0 , . . . , Gn and edges ei ∈ Gi such that G 0 =  G, Gn ' X, and Gi+1 =  Gi/ei for all i < n.

Proof . Induction on  |G| − |X|.

¤

If  G =  M X  is a subgraph of another graph  Y , we call  X  a  minor  of  Y

and write  X  4  Y . Note that every subgraph of a graph is also its minor; minor;  4

in particular, every graph is its own minor. By Proposition 1.7.1, any

minor of a graph can be obtained from it by first deleting some vertices and edges, and then contracting some further edges. Conversely, any

graph obtained from another by repeated deletions and contractions (in

any order) is its minor: this is clear for one deletion or contraction, and follows for several from the transitivity of the minor relation (Proposition

1.7.3).

If we replace the edges of  X  with independent paths between their ends (so that none of these paths has an inner vertex on another path

or in  X), we call the graph  G  obtained a  subdivision  of  X  and write subdivision

T X

G =  T X.7 If  G =  T X  is the subgraph of another graph  Y , then  X  is a topological minor  of  Y (Fig. 1.7.3).

topological

minor

7 So again  T X  denotes an entire class of graphs: all those which, viewed as a topological space in the obvious way, are homeomorphic to  X. The  T  in  T X  stands for ‘topological’.

18

1. The Basics

G

X

Y

Fig. 1.7.3. Y ⊇ G =  T X, so  X  is a topological minor of  Y

If  G =  T X, we view  V ( X) as a subset of  V ( G) and call these vertices branch

the  branch vertices  of  G; the other vertices of  G  are its  subdividing vertices

vertices. Thus, all subdividing vertices have degree 2, while the branch vertices retain their degree from  X.

[ 4.4.2 ]

Proposition 1.7.2.

[ 8.3.1 ]

(i)  Every T X is also an M X (Fig. 1.7.4); thus, every topological

minor of a graph is also its (ordinary) minor.

(ii)  If ∆( X) 6 3 , then every M X contains a T X; thus, every minor with maximum degree at most  3  of a graph is also its topological minor.

¤

Fig. 1.7.4.  A subdivision of  K 4 viewed as an  M K 4

[ 12.4.1 ]

Proposition 1.7.3.  The minor relation  4  and the topological-minor relation are partial orderings on the class of finite graphs, i.e. they are reflexive, antisymmetric and transitive.

¤

1.8 Euler tours

Any mathematician who happens to find himself in the East Prussian

city of Königsberg (and in the 18th century) will lose no time to follow the great Leonhard Euler’s example and inquire about a round trip through

1.8 Euler tours

19

Fig. 1.8.1.  The bridges of Königsberg (anno 1736)

the old city that traverses each of the bridges shown in Figure 1.8.1

exactly once.

Thus inspired,8 let us call a closed walk in a graph an  Euler tour  if it traverses every edge of the graph exactly once. A graph is  Eulerian  if Eulerian

it admits an Euler tour.

Fig. 1.8.2.  A graph formalizing the bridge problem

Theorem 1.8.1. (Euler 1736)

A connected graph is Eulerian if and only if every vertex has even degree.

[ 2.1.5 ]

[ 10.3.3 ]

Proof . The degree condition is clearly necessary: a vertex appearing  k times in an Euler tour (or  k + 1 times, if it is the starting and finishing vertex and as such counted twice) must have degree 2 k.

8 Anyone to whom such inspiration seems far-fetched, even after contemplating Figure 1.8.2, may seek consolation in the  multigraph  of Figure 1.10.1.

20

1. The Basics

Conversely, let  G  be a connected graph with all degrees even, and let

W =  v 0 e 0  . . . è− 1 v`

be a longest walk in  G  using no edge more than once. Since  W  cannot be extended, it already contains all the edges at  v`. By assumption, the number of such edges is even. Hence  v` =  v 0, so  W  is a closed walk.

Suppose  W  is not an Euler tour. Then  G  has an edge  e  outside  W

but incident with a vertex of  W , say  e =  uvi. (Here we use the connectedness of  G, as in the proof of Proposition 1.4.1.) Then the walk ueviei . . . è− 1 vè 0  . . . ei− 1 vi

is longer than  W , a contradiction.

¤

1.9 Some linear algebra

Let  G = ( V, E) be a graph with  n  vertices and  m  edges, say  V =

vertex

space

{ v 1 , . . . , vn }  and  E =  { e 1 , . . . , em }. The  vertex space V( G) of  G  is the V( G)

vector space over the 2-element field F2 =  {  0 ,  1  }  of all functions  V →  F2.

Every element of  V( G) corresponds naturally to a subset of  V , the set of those vertices to which it assigns a 1, and every subset of  V  is uniquely represented in  V( G) by its indicator function. We may thus think of V( G) as the power set of  V  made into a vector space: the sum  U +  U0

+

of two vertex sets  U, U 0 ⊆ V  is their symmetric difference (why?), and U =  −U  for all  U ⊆ V . The zero in  V( G), viewed in this way, is the empty (vertex) set  ∅. Since  { { v 1  }, . . . , { vn } }  is a basis of  V( G), its standard basis, we have dim  V( G) =  n.

In the same way as above, the functions  E →  F2 form the  edge edge space

space E( G) of  G: its elements are the subsets of  E, vector addition E( G)

amounts to symmetric difference,  ∅ ⊆ E  is the zero, and  F =  −F  for standard

all  F ⊆ E. As before,  { { e 1  }, . . . , { em } }  is the  standard basis  of  E( G), basis

and dim  E( G) =  m.

Since the edges of a graph carry its essential structure, we shall

mostly be concerned with the edge space. Given two edge sets  F, F 0 ∈

E( G) and their coefficients  λ 1 , . . . , λm  and  λ0 1 , . . . , λ0m  with respect to the standard basis, we write

hF, F 0i

hF, F 0i :=  λ 1 λ0

∈  F

1 +  . . . +  λmλ0m

2  .

Note that  hF, F 0i = 0 may hold even when  F =  F 0 6=  ∅: indeed, hF, F 0i = 0 if and only if  F  and  F 0  have an even number of edges

1.9 Some linear algebra

21

in common. Given a subspace  F  of  E( G), we write

©

ª

F⊥ :=  D ∈ E( G)  | hF, Di = 0 for all  F ∈ F .

F⊥

This is again a subspace of  E( G) (the space of all vectors solving a certain set of linear equations—which?), and we have

dim  F + dim  F⊥ =  m .

cycle space

The  cycle space C =  C( G) is the subspace of  E( G) spanned by all C( G)

the cycles in  G—more precisely, by their edge sets.9 The dimension of C( G) is the  cyclomatic number  of  G.

Proposition 1.9.1.  The induced cycles in G generate its entire cycle

[ 3.2.3 ]

space.

Proof . By definition of  C( G) it suffices to show that the induced cycles in  G  generate every cycle  C ⊆ G  with a chord  e. This follows at once by induction on  |C|: the two cycles in  C +  e  with  e  but no other edge in common are shorter than  C, and their symmetric difference is precisely  C.

¤

Proposition 1.9.2.  An edge set F ⊆ E lies in C( G)  if and only if every

[ 4.5.1 ]

vertex of ( V, F )  has even degree.

Proof . The forward implication holds by induction on the number of cycles needed to generate  F , the backward implication by induction on the number of cycles in ( V, F ).

¤

If  { V 1 , V 2  }  is a partition of  V , the set  E( V 1 , V 2) of all the edges of G crossing  this partition is called a  cut . Recall that for  V 1 =  { v }  this cut

cut is denoted by  E( v).

Proposition 1.9.3.  Together with ∅, the cuts in G form a subspace C∗

[ 4.6.3 ]

of E( G) . This space is generated by cuts of the form E( v) .

Proof . Let  C∗  denote the set of all cuts in  G, together with  ∅. To prove that  C∗  is a subspace, we show that for all  D, D0 ∈ C∗  also  D +  D0

(=  D − D0) lies in  C∗. Since  D +  D =  ∅ ∈ C∗  and  D +  ∅ =  D ∈ C∗, we may assume that  D  and  D0  are distinct and non-empty.

Let

{ V 1 , V 2  }  and  { V 0

}

1  , V 0

2

be the corresponding partitions of  V .

Then

D +  D0  consists of all the edges that cross one of these partitions but not the other (Fig. 1.9.1). But these are precisely the edges between ( V 1  ∩ V 0 1)  ∪ ( V 2  ∩ V 0 2) and ( V 1  ∩ V 0 2)  ∪ ( V 2  ∩ V 0 1), and by  D 6=  D0  these two 9 For simplicity, we shall not normally distinguish between cycles and their edge sets in connection with the cycle space.

22

1. The Basics

V 1

V 2

V 0 1

D0

V 0 2

D

Fig. 1.9.1.  Cut edges in  D +  D0

sets form another partition of  V . Hence  D +  D0 ∈ C∗, and  C∗  is indeed a subspace of  E( G).

Our second assertion, that the cuts  E( v) generate all of  C∗, follows from the fact that every edge  xy ∈ G  lies in exactly two such cuts (in  E( x) and in  E( y)); thus every partition  { V 1 , V 2  }  of  V  satisfies  E( V 1 , V 2) =

P

E( v).

¤

v ∈V 1

The subspace  C∗ =:  C∗( G) of  E( G) from Proposition 1.9.3 will be cut space

called the  cut space  of  G. It is not difficult to find among the cuts C∗( G)

E( v) an explicit basis for  C∗( G), and thus to determine its dimension (exercise); together with Theorem 1.9.5 this yields an independent proof of Theorem 1.9.6.

The following lemma will be useful when we study the duality of

plane graphs in Chapter 4.6:

[ 4.6.2 ]

Lemma 1.9.4.  The minimal cuts in a connected graph generate its entire cut space.

Proof . Note first that a cut in a connected graph  G = ( V, E) is minimal if and only if both sets in the corresponding partition of  V  are connected in  G. Now consider any connected subgraph  C ⊆ G. If  D  is a component of  G − C, then also  G − D  is connected (Fig. 1.9.2); the edges between  D

D

G − D

C

Fig. 1.9.2. G − D  is connected, and  E( C, D) a minimal cut

1.9 Some linear algebra

23

and  G − D  thus form a minimal cut. By choice of  D, this cut is precisely the set  E( C, D) of all  C– D  edges in  G.

To prove the lemma, let a partition  { V 1 , V 2  }  of  V  be given, and consider a component  C  of  G [  V 1 ]. Then  E( C, V 2) =  E( C, G − C) is the disjoint union of the edge sets  E( C, D) over all components  D  of G − C, and is thus the disjoint union of minimal cuts (see above). Now the disjoint union of all these edge sets  E( C, V 2), taken over all the components  C  of  G [  V 1 ], is precisely our cut  E( V 1 , V 2). So this cut is generated by minimal cuts, as claimed.

¤

Theorem 1.9.5.  The cycle space C and the cut space C∗ of any graph satisfy

C =  C∗⊥ and C∗ =  C⊥ .

Proof . Let us consider a graph  G = ( V, E). Clearly, any cycle in  G  has an even number of edges in each cut. This implies  C ⊆ C∗⊥.

Conversely, recall from Proposition 1.9.2 that for every edge set F /

∈ C  there exists a vertex  v  incident with an odd number of edges in  F .

Then  hE( v) , F i = 1, so  E( v)  ∈ C∗  implies  F /

∈ C∗⊥. This completes the

proof of  C =  C∗⊥.

To prove  C∗ =  C⊥, it now suffices to show  C∗ = ( C∗⊥) ⊥. Here C∗ ⊆ ( C∗⊥) ⊥  follows directly from the definition of  ⊥. But since dim  C∗ + dim  C∗⊥ =  m = dim  C∗⊥ + dim ( C∗⊥) ⊥, C∗  has the same dimension as ( C∗⊥) ⊥, so  C∗ = ( C∗⊥) ⊥  as claimed.

¤

Theorem 1.9.6.  Every connected graph G with n vertices and m edges

[ 4.5.1 ]

satisfies

dim  C( G) =  m − n + 1  and  dim  C∗( G) =  n −  1  .

Proof . Let  G = ( V, E). As dim  C + dim  C∗ =  m  by Theorem 1.9.5, it

(1.5.1)

(1.5.3)

suffices to find  m − n + 1 linearly independent vectors in  C  and  n −  1

linearly independent vectors in  C∗: since these numbers add up to  m, neither the dimension of  C  nor that of  C∗  can then be strictly greater.

Let  T  be a spanning tree in  G. By Corollary 1.5.3,  T  has  n −  1

edges, so  m − n + 1 edges of  G  lie outside  T . For each of these  m − n + 1

edges  e ∈ E  r  E( T ), the graph  T +  e  contains a cycle  Ce (see Fig. 1.6.3

and Theorem 1.5.1 (iv)). Since none of the edges  e  lies on  Ce0  for  e0 6=  e, these  m − n + 1 cycles are linearly independent.

For each of the  n −  1 edges  e ∈ T , the graph  T − e  has exactly two components (Theorem 1.5.1 (iii)), and the set  De  of edges in  G  between these components form a cut (Fig.1.9.3). Since none of the edges  e ∈ T

lies in  De0  for  e0 6=  e, these  n −  1 cuts are linearly independent.

¤

24

1. The Basics

e

Fig. 1.9.3.  The cut  De

incidence

The  incidence matrix B = ( bij) n×m  of a graph  G = ( V, E) with matrix

V =  { v 1 , . . . , vn }  and  E =  { e 1 , . . . , em }  is defined over F2 by n 1 if  v

b

i ∈ ej

ij :=

0

otherwise.

As usual, let  Bt  denote the transpose of  B. Then  B  and  Bt  define linear maps  B:  E( G)  →V( G) and  Bt:  V( G)  →E( G) with respect to the standard bases.

Proposition 1.9.7.

(i)  The kernel of B is C( G) .

(ii)  The image of Bt is C∗( G) .

¤

adjacency

The  adjacency matrix A = ( aij) n×n  of  G  is defined by matrix

n 1 if  v

a

ivj ∈ E

ij :=

0

otherwise.

Our last proposition establishes a simple connection between  A  and  B

(now viewed as real matrices). Let  D  denote the real diagonal matrix ( dij) n×n  with  dii =  d( vi) and  dij = 0 otherwise.

Proposition 1.9.8.  BBt =  A +  D.

¤

1.10 Other notions of graphs

25

1.10 Other notions of graphs

For completeness, we now mention a few other notions of graphs which

feature less frequently or not at all in this book.

A  hypergraph  is a pair ( V, E) of disjoint sets, where the elements  hypergraph of  E  are non-empty subsets (of any cardinality) of  V . Thus, graphs are special hypergraphs.

A  directed graph (or  digraph) is a pair ( V, E) of disjoint sets (of directed

graph

vertices  and  edges) together with two maps init:  E → V  and ter:  E → V

assigning to every edge  e  an  initial vertex  init( e) and a  terminal vertex init( e)

ter( e). The edge  e  is said to be  directed from  init( e)  to  ter( e). Note that ter( e)

a directed graph may have several edges between the same two vertices

x, y. Such edges are called  multiple edges; if they have the same direction (say from  x  to  y), they are  parallel . If init( e) = ter( e), the edge  e  is called a  loop.

loop

A directed graph  D  is an  orientation  of an (undirected) graph  G  if orientation

V ( D) =  V ( G) and  E( D) =  E( G), and if  {  init( e) ,  ter( e)  } =  { x, y }  for oriented

every edge  e =  xy. Intuitively, such an  oriented graph  arises from an graph

undirected graph simply by directing every edge from one of its ends to the other. Put differently, oriented graphs are directed graphs without loops or multiple edges.

A  multigraph  is a pair ( V, E) of disjoint sets (of  vertices  and  edges) multigraph

together with a map  E → V ∪ [ V ]2 assigning to every edge either one or two vertices, its  ends. Thus, multigraphs too can have loops and multiple edges: we may think of a multigraph as a directed graph whose

edge directions have been ‘forgotten’. To express that  x  and  y  are the ends of an edge  e  we still write  e =  xy, though this no longer determines e  uniquely.

A graph is thus essentially the same as a multigraph without loops

or multiple edges. Somewhat surprisingly, proving a graph theorem more

generally for multigraphs may, on occasion, simplify the proof. Moreover, there are areas in graph theory (such as plane duality; see Chapters 4.6

and 6.5) where multigraphs arise more naturally than graphs, and where

any restriction to the latter would seem artificial and be technically

complicated. We shall therefore consider multigraphs in these cases, but without much technical ado: terminology introduced earlier for graphs

will be used correspondingly.

Two differences, however, should be pointed out. First, a multi-

graph may have cycles of length 1 or 2: loops, and pairs of multiple

edges (or  double edges). Second, the notion of edge contraction is simpler in multigraphs than in graphs. If we contract an edge  e =  xy  in a multigraph  G = ( V, E) to a new vertex  ve, there is no longer a need to delete any edges other than  e  itself: edges parallel to  e  become loops at  ve, while edges  xv  and  yv  become parallel edges between  ve  and  v

(Fig. 1.10.1). Thus, formally,  E( G/e) =  E  r  { e }, and only the incidence

26

1. The Basics

map  e0 7→ {  init( e0) ,  ter( e0)  }  of  G  has to be adjusted to the new vertex set in  G/e. The notion of a minor adapts to multigraphs accordingly.

ve

e

G

G/e

Fig. 1.10.1.  Contracting the edge  e  in the multigraph corresponding to Fig. 1.8.1

Finally, it should be pointed out that authors who usually work with

multigraphs tend to call them graphs; in their terminology, our graphs

would be called simple graphs.

Exercises

1.  −  What is the number of edges in a  Kn?

2.

Let  d ∈  N and  V :=  {  0 ,  1  }d; thus,  V  is the set of all 0–1 sequences of length  d. The graph on  V  in which two such sequences form an edge if and only if they differ in exactly one position is called the  d-dimensional cube. Determine the average degree, number of edges, diameter, girth and circumference of this graph.

(Hint for circumference. Induction on  d.)

3.

Let  G  be a graph containing a cycle  C, and assume that  G  contains a path of length at least  k  between two vertices of  C. Show that  G

√

contains a cycle of length at least

k. Is this best possible?

4.  −  Is the bound in Proposition 1.3.2 best possible?

5.

Show that rad( G) 6 diam( G) 6 2 rad( G) for every graph  G.

6.+ Assuming that  d > 2 and  k > 3, improve the bound in Proposition

1.3.3 to  dk.

7.  −  Show that the components of a graph partition its vertex set. (In other words, show that every vertex belongs to exactly one component.)

8.  −  Show that every 2-connected graph contains a cycle.

9.

(i) −  Determine  κ( G) and  λ( G) for  G =  P k, Ck, Kk, Km,n ( k, m, n > 3).

(ii)+ Determine the connectivity of the  n-dimensional cube (defined in Exercise 2).

(Hint for (ii). Induction on  n.)

10.

Show that  κ( G) 6  λ( G) 6  δ( G) for every non-trivial graph  G.

Exercises

27

11.  −  Is there a function  f : N  →  N such that, for all  k ∈  N, every graph of minimum degree at least  f ( k) is  k-connected?

12.

Let  α, β  be two graph invariants with positive integer values. Formalize the two statements below, and show that each implies the other:

(i)  α  is bounded above by a function of  β;

(ii)  β  can be forced up by making  α  large enough.

Show that the statement

(iii)  β  is bounded below by a function of  α

is not equivalent to (i) and (ii). Which small change would make it so?

13.+ What is the deeper reason behind the fact that the proof of Theorem

1.4.2 is based on an assumption of the form  m >  cn − b  rather than just on a lower bound for the average degree?

14.

Prove Theorem 1.5.1.

15.

Show that any tree  T  has at least ∆( T ) leaves.

16.

Show that the ‘tree-order’ associated with a rooted tree  T  is indeed a partial order on  V ( T ), and verify the claims made about this partial order in the text.

17.

Let  G  be a connected graph, and let  r ∈ G  be a vertex. Starting from  r, move along the edges of  G, going whenever possible to a vertex not visited so far. If there is no such vertex, go back along the edge by which the current vertex was first reached (unless the current vertex

is  r; then stop). Show that the edges traversed form a normal spanning tree in  G  with root  r.

(This procedure has earned those trees the name of  depth-first search trees.)

18.

Let  T  be a set of subtrees of a tree  T . Assume that the trees in  T  have pairwise non-empty intersection. Show that their overall intersection

T  T  is non-empty.

19.

Show that every automorphism of a tree fixes a vertex or an edge.

20.

Are the partition classes of a regular bipartite graph always of the same size?

21.

Show that a graph is bipartite if and only if every  induced  cycle has even length.

22.

Find a function  f : N  →  N such that, for all  k ∈  N, every graph of average degree at least  f ( k) has a bipartite subgraph of minimum degree at least  k.

23.

Show that the minor relation 4 defines a partial ordering on any set of (finite) graphs. Is the same true for infinite graphs?

24.  −  Show that the elements of the cycle space of a graph  G  are precisely the unions of the edges sets of edge-disjoint cycles in  G.

28

1. The Basics

25.

Given a graph  G, find among all cuts of the form  E( v) a basis for the cut space of  G.

26.

Prove that the cycles and the cuts in a graph together generate its

entire edge space, or find a counterexample.

27.

Give a direct proof of the fact that the cycles  Ce  defined in the proof of Theorem 1.9.6 generate the cycle space.

28.

Give a direct proof of the fact that the cuts  De  defined in the proof of

Theorem 1.9.6 generate the cut space.

29.

What are the dimensions of the cycle and the cut space of a graph with

k  components?

Notes

The terminology used in this book is mostly standard. Alternatives do exist, of course, and some of these are stated when a concept is first defined. There is one small point where our notation deviates slightly from standard usage.

Whereas complete graphs, paths, cycles etc. of given order are mostly denoted by  Kn,  Pk,  C`  and so on, we use superscripts instead of subscripts. This has the advantage of leaving the variables  K,  P ,  C  etc. free for ad-hoc use: we may now enumerate components as  C 1 , C 2 , . . . , speak of paths  P 1 , . . . , Pk, and so on—without any danger of confusion.

Theorem10 1.4.2 is due to W. Mader, Existenz  n-fach zusammenhängen-der Teilgraphen in Graphen genügend großer Kantendichte,  Abh. Math. Sem.

Univ. Hamburg 37 (1972) 86–97. Theorem 1.8.1 is from L. Euler, Solutio problematis ad geometriam situs pertinentis,  Comment. Acad. Sci. I. Petro-politanae 8 (1736), 128–140.

Of the large subject of algebraic methods in graph theory, Section 1.9 does not claim to convey an adequate impression. The standard monograph here is N.L. Biggs,  Algebraic Graph Theory (2nd edn.), Cambridge University Press 1993. Another comprehensive account is given by C.D. Godsil & G.F. Royle, Algebraic Graph Theory, in preparation. Surveys on the use of algebraic methods can also be found in the  Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995.

10 In the interest of readability, the end-of-chapter notes in this book give references only for Theorems, and only in cases where these references cannot be found in a monograph or survey cited for that chapter.

2

Matching

Suppose we are given a graph and are asked to find in it as many in-

dependent edges as possible. How should we go about this? Will we

be able to pair up all its vertices in this way? If not, how can we be

sure that this is indeed impossible? Somewhat surprisingly, this basic

problem does not only lie at the heart of numerous applications, it also gives rise to some rather interesting graph theory.

A set  M  of independent edges in a graph  G = ( V, E) is called a matching.  M  is a matching  of U ⊆ V  if every vertex in  U  is incident matching

with an edge in  M . The vertices in  U  are then called  matched (by  M ); matched

vertices not incident with any edge of  M  are  unmatched .

A  k-regular spanning subgraph is called a  k- factor . Thus, a sub-factor

graph  H ⊆ G  is a 1-factor of  G  if and only if  E( H) is a matching of  V .

The problem of how to characterize the graphs that have a 1-factor, i.e.

a matching of their entire vertex set, will be our main theme in this

chapter.

2.1 Matching in bipartite graphs

For this whole section, we let  G = ( V, E) be a fixed bipartite graph with G = ( V, E)

bipartition  { A, B }. Vertices denoted as  a, a0  etc. will be assumed to lie A, B

in  A, vertices denoted as  b  etc. will lie in  B.

a, b etc.

How can we find a matching in  G  with as many edges as possible?

Let us start by considering an arbitrary matching  M  in  G. A path in  G

which starts in  A  at an unmatched vertex and then contains, alternately, alternating

edges from  E  r  M  and from  M , is an  alternating path  with respect to  M .

path

An alternating path  P  that ends in an unmatched vertex of  B  is called augment-an  augmenting path (Fig. 2.1.1), because we can use it to turn  M  into ing path

a larger matching: the symmetric difference of  M  with  E( P ) is again a

30

2. Matching

P

M 0

M

A

B

A

B

Fig. 2.1.1.  Augmenting the matching  M  by the alternating path  P

matching (consider the edges at a given vertex), and the set of matched vertices is increased by two, the ends of  P .

Alternating paths play an important role in the practical search for

large matchings. In fact, if we start with any matching and keep applying augmenting paths until no further such improvement is possible, the

matching obtained will always be an optimal one, a matching with the

largest possible number of edges (Exercise 1). The algorithmic problem of finding such matchings thus reduces to that of finding augmenting

paths—which is an interesting and accessible algorithmic problem.

Our first theorem characterizes the maximal cardinality of a matching

in  G  by a kind of duality condition. Let us call a set  U ⊆ V  a  cover  of  E

vertex

(or a  vertex cover  of  G) if every edge of  G  is incident with a vertex in  U .

cover

Theorem 2.1.1. (König 1931)

The maximum cardinality of a matching in G is equal to the minimum

cardinality of a vertex cover.

M

Proof . Let  M  be a matching in  G  of maximum cardinality. From every edge in  M  let us choose one of its ends: its end in  B  if some alternating path ends in that vertex, and its end in  A  otherwise (Fig. 2.1.2). We U

shall prove that the set  U  of these  |M |  vertices covers  G; since any vertex cover of  G  must cover  M , there can be none with fewer than  |M |  vertices, and so the theorem will follow.

U ∩ B

U ∩ A

Fig. 2.1.2.  The vertex cover  U

2.1 Matching in bipartite graphs

31

Let  ab ∈ E  be an edge; we show that either  a  or  b  lies in  U . If ab ∈ M , this holds by definition of  U , so we assume that  ab /

∈ M . Since

M  is a maximal matching, it contains an edge  a0b0  with  a =  a0  or  b =  b0.

In fact, we may assume that  a =  a0: for if  a  is unmatched (and  b =  b0), then  ab  is an alternating path, and so the end of  a0b0 ∈ M  chosen for U  was the vertex  b0 =  b. Now if  a0 =  a  is not in  U , then  b0 ∈ U , and some alternating path  P  ends in  b0. But then there is also an alternating path  P 0  ending in  b: either  P 0 :=  P b (if  b ∈ P ) or  P 0 :=  P b0a0b. By the maximality of  M , however,  P 0  is not an augmenting path. So  b  must be matched, and was chosen for  U  from the edge of  M  containing it.

¤

Let us return to our main problem, the search for some necessary

and sufficient conditions for the existence of a 1-factor. In our present case of a bipartite graph, we may as well ask more generally when  G

contains a matching of  A; this will define a 1-factor of  G  if  |A| =  |B|, a condition that has to hold anyhow if  G  is to have a 1-factor.

A condition clearly necessary for the existence of a matching of  A

is that every subset of  A  has enough neighbours in  B, i.e. that marriage

|N( S) | >  |S|

for all  S ⊆ A.

condition

The following  marriage theorem  says that this obvious necessary condition is in fact sufficient:

marriage

Theorem 2.1.2. (Hall 1935)

theorem

G contains a matching of A if and only if |N ( S) | >  |S| for all S ⊆ A.

We give three proofs for the non-trivial implication of this theorem, i.e.

that the ‘marriage condition’ implies the existence of a matching of  A.

The first of these is based on K¨

onig’s theorem; the second is a direct

constructive proof by augmenting paths; the third will be an independent proof from first principles.

First proof. If  G  contains no matching of  A, then by Theorem 2.1.1

it has a cover  U  consisting of fewer than  |A|  vertices, say  U =  A0 ∪ B0

with  A0 ⊆ A  and  B0 ⊆ B. Then

|A0| +  |B0| =  |U| < |A| ,

and hence

|B0| < |A| − |A0| =  |A  r  A0|

(Fig. 2.1.3). By definition of  U , however,  G  has no edges between  A  r  A0

and  B  r  B0, so

|N( A  r  A0) |  6  |B0| < |A  r  A0|

and the marriage condition fails for  S :=  A  r  A0.

¤

32

2. Matching

A0

B0

Fig. 2.1.3.  A cover by fewer than  |A|  vertices

M

Second proof. Consider a matching  M  of  G  that leaves a vertex of  A unmatched; we shall construct an augmenting path with respect to  M .

Let  a 0 , b 1 , a 1 , b 2 , a 2 , . . .  be a maximal sequence of distinct vertices  ai ∈ A and  bi ∈ B  satisfying the following conditions for all  i > 1 (Fig. 2.1.4):

(i)  a 0 is unmatched;

f ( i)

(ii)  bi  is adjacent to some vertex  af( i)  ∈ { a 0 , . . . , ai− 1  }; (iii)  aibi ∈ M .

By the marriage condition, our sequence cannot end in a vertex of  A: the  i  vertices  a 0 , . . . , ai− 1 together have at least  i  neighbours in  B, so we can always find a new vertex  bi 6=  b 1 , . . . , bi− 1 that satisfies (ii). Let k

bk ∈ B  be the last vertex of the sequence. By (i)–(iii),

P

P :=  bkaf( k) bf( k) af 2( k) bf 2( k) af 3( k)  . . . afr( k) with  f r( k) = 0 is an alternating path.

a 2

b 2

a 0

b 1

a 1

b 3

a 3

b 4

a 4

b 5

Fig. 2.1.4.  Proving the marriage theorem by alternating paths What is it that prevents us from extending our sequence further?

If  bk  is matched, say to  a, we can indeed extend it by setting  ak :=  a, unless  a =  ai  with 0  < i < k, in which case (iii) would imply  bk =  bi with a contradiction. So  bk  is unmatched, and hence  P  is an augmenting path between  a 0 and  bk.

¤

2.1 Matching in bipartite graphs

33

Third proof. We apply induction on  |A|. For  |A| = 1 the assertion is true. Now let  |A| > 2, and assume that the marriage condition is sufficient for the existence of a matching of  A  when  |A|  is smaller.

If  |N ( S) | >  |S| + 1 for every non-empty set  S $  A, we pick an edge ab ∈ G  and consider the graph  G0 :=  G − { a, b }. Then every non-empty set  S ⊆ A  r  { a }  satisfies

|NG0( S) | >  |NG( S) | −  1 >  |S| , so by the induction hypothesis  G0  contains a matching of  A  r  { a }. Together with the edge  ab, this yields a matching of  A  in  G.

Suppose now that  A  has a non-empty proper subset  A0  with  |B0| =

A0, B0

|A0|  for  B0 :=  N( A0). By the induction hypothesis,  G0 :=  G [  A0 ∪ B0 ]

G0

contains a matching of  A0. But  G − G0  satisfies the marriage condition too: for any set  S ⊆ A  r  A0  with  |NG−G0( S) | < |S|  we would have

|NG( S ∪ A0) | < |S ∪ A0|, contrary to our assumption. Again by induction,  G − G0  contains a matching of  A  r  A0. Putting the two matchings together, we obtain a matching of  A  in  G.

¤

Corollary 2.1.3.  If |N ( S) | >  |S| − d for every set S ⊆ A and some

[ 2.2.3 ]

fixed d ∈  N , then G contains a matching of cardinality |A| − d.

Proof . We add  d  new vertices to  B, joining each of them to all the vertices in  A. By the marriage theorem the new graph contains a matching of  A, and at least  |A|− d  edges in this matching must be edges of  G.

¤

Corollary 2.1.4.  If G is k-regular with k > 1 , then G has a 1-factor.

Proof . If  G  is  k-regular, then clearly  |A| =  |B|; it thus suffices to show by

Theorem 2.1.2 that  G  contains a matching of  A. Now every set  S ⊆ A is joined to  N ( S) by a total of  k |S|  edges, and these are among the k |N ( S) |  edges of  G  incident with  N ( S). Therefore  k |S|  6  k |N ( S) |, so G  does indeed satisfy the marriage condition.

¤

Despite its seemingly narrow formulation, the marriage theorem

counts among the most frequently applied graph theorems, both out-

side graph theory and within. Often, however, recasting a problem in

the setting of bipartite matching requires some clever adaptation. As a simple example, we now use the marriage theorem to derive one of the earliest results of graph theory, a result whose original proof is not all that simple, and certainly not short:

Corollary 2.1.5. (Petersen 1891)

Every regular graph of positive even degree has a 2-factor.

34

2. Matching

(1.8.1)

Proof . Let  G  be any 2 k-regular graph ( k > 1), without loss of generality connected. By Theorem 1.8.1,  G  contains an Euler tour  v 0 e 0  . . . è− 1 v`, with  v` =  v 0. We replace every vertex  v  by a pair ( v−, v+), and every edge  ei =  vivi+1 by the edge  v+ v− (Fig. 2.1.5). The resulting bipartite i

i+1

graph  G0  is  k-regular, so by Corollary 2.1.4 it has a 1-factor. Collapsing every vertex pair ( v−, v+) back into a single vertex  v, we turn this 1-factor of  G0  into a 2-factor of  G.

¤

v−

v

v+

Fig. 2.1.5.  Splitting vertices in the proof of Corollary 2.1.5

2.2 Matching in general graphs

CG

Given a graph  G, let us denote by  CG  the set of its components, and by q( G)

q( G) the number of its  odd components, those of odd order. If  G  has a 1-factor, then clearly

Tutte’s

condition

q( G − S) 6  |S|

for all  S ⊆ V ( G),

since every odd component of  G − S  will send a factor edge to  S.

S

S

G

HS

Fig. 2.2.1.  Tutte’s condition  q( G − S) 6  |S|  for  q = 3, and the contracted graph  HS  from Theorem 2.2.3.

Again, this obvious necessary condition for the existence of a 1-factor is also sufficient:

2.2 Matching in general graphs

35

Theorem 2.2.1. (Tutte 1947)

A graph G has a 1-factor if and only if q( G − S) 6  |S| for all S ⊆ V ( G) .

Proof . Let  G = ( V, E) be a graph without a 1-factor. Our task is to V, E

find a  bad set S ⊆ V , one that violates Tutte’s condition.

bad set

We may assume that  G  is edge-maximal without a 1-factor. Indeed, if  G0  is obtained from  G  by adding edges and  S ⊆ V  is bad for  G0, then S  is also bad for  G: any odd component of  G0 − S  is the union of components of  G − S, and one of these must again be odd.

What does  G  look like? Clearly,  if G  contains a bad set  S  then, by its edge-maximality and the trivial forward implication of the theorem, all the components of G − S are complete and every vertex

( ∗)

s ∈ S is adjacent to all the vertices of G − s.

But also conversely, if a set  S ⊆ V  satisfies ( ∗) then either  S  or the empty set must be bad: if  S  is not bad we can join the odd components of  G − S  disjointly to  S  and pair up all the remaining vertices—unless

|G|  is odd, in which case  ∅  is bad.

So it suffices to prove that  G  has a set  S  of vertices satisfying ( ∗).

Let  S  be the set of vertices that are adjacent to every other vertex. If S

this set  S  does not satisfy ( ∗), then some component of  G − S  has nonadjacent vertices  a, a0. Let  a, b, c  be the first three vertices on a shortest a, b, c

a– a0  path in this component; then  ab, bc ∈ E  but  ac /

∈ E. Since  b /

∈ S,

there is a vertex  d ∈ V  such that  bd /

∈ E. By the maximality of  G, there

d

is a matching  M 1 of  V  in  G +  ac, and a matching  M 2 of  V  in  G +  bd.

M 1 , M 2

a

d

1

b

2

2

1

C

. . .

c

1

P

Fig. 2.2.2.  Deriving a contradiction if  S  does not satisfy ( ∗) Let  P =  d . . . v  be a maximal path in  G  starting at  d  with an edge v

from  M 1 and containing alternately edges from  M 1 and  M 2 (Fig. 2.2.2).

If the last edge of  P  lies in  M 1, then  v =  b, since otherwise we could continue  P . Let us then set  C :=  P +  bd. If the last edge of  P  lies in  M 2, then by the maximality of  P  the  M 1-edge at  v  must be  ac, so  v ∈ { a, c }; then let  C  be the cycle  dP vbd. In each case,  C  is an even cycle with every other edge in  M 2, and whose only edge not in  E  is  bd. Replacing in  M 2 its edges on  C  with the edges of  C − M 2, we obtain a matching of  V  contained in  E, a contradiction.

¤

36

2. Matching

Corollary 2.2.2. (Petersen 1891)

Every bridgeless cubic graph has a 1-factor.

Proof . We show that any bridgeless cubic graph  G  satisfies Tutte’s condition. Let  S ⊆ V ( G) be given, and consider an odd component  C  of G − S. Since  G  is cubic, the degrees (in  G) of the vertices in  C  sum to an odd number, but only an even part of this sum arises from edges of  C.

So  G  has an odd number of  S– C  edges, and therefore has at least 3 such edges (since  G  has no bridge). The total number of edges between  S  and G − S  thus is at least 3 q( G − S). But it is also at most 3 |S|, because  G

is cubic. Hence  q( G − S) 6  |S|, as required.

¤

In order to shed a little more light on the techniques used in match-

ing theory, we now give a second proof of Tutte’s theorem. In fact, we shall prove a slightly stronger result, a result that places a structure interesting from the matching point of view on an  arbitrary  graph. If the graph happens to satisfy the condition of Tutte’s theorem, this structure will at once yield a 1-factor.

factor-

A graph  G = ( V, E) is called  factor-critical  if  G 6=  ∅  and  G − v critical

has a 1-factor for every vertex  v ∈ G. Then  G  itself has no 1-factor, matchable

because it has odd order. We call a vertex set  S ⊆ V matchable to

G − S  if the (bipartite1) graph  HS, which arises from  G  by contracting the components  C ∈ CG−S  to single vertices and deleting all the edges HS

inside  S, contains a matching of  S. (Formally,  HS  is the graph with vertex set  S ∪CG−S  and edge set  { sC | ∃ c ∈ C :  sc ∈ E }; see Fig. 2.2.1.)

Theorem 2.2.3.  Every graph G = ( V, E)  contains a vertex set S with the following two properties:

(i)  S is matchable to G − S;

(ii)  every component of G − S is factor-critical.

Given any such set S, the graph G contains a 1-factor if and only if

|S| =  |CG−S|.

For any given  G, the assertion of Tutte’s theorem follows easily from this result. Indeed, by (i) and (ii) we have  |S|  6  |CG−S| =  q( G − S) (since factor-critical graphs have odd order); thus Tutte’s condition of q( G − S) 6  |S|  implies  |S| =  |CG−S|, and the existence of a 1-factor follows from the last statement of Theorem 2.2.3.

(2.1.3)

Proof of Theorem 2.2.3. Note first that the last assertion of the theorem follows at once from the assertions (i) and (ii): if  G  has a 1-factor, we have  q( G − S) 6  |S|  and hence  |S| =  |CG−S|  as above; 1 except for the—permitted—case that  S  or  CG−S  is empty

2.2 Matching in general graphs

37

conversely if  |S| =  |CG−S|, then the existence of a 1-factor follows straight from (i) and (ii).

We now prove the existence of a set  S  satisfying (i) and (ii). We apply induction on  |G|. For  |G| = 0 we may take  S =  ∅. Now let  G

be given with  |G| >  0, and assume the assertion holds for graphs with fewer vertices.

Let  d  be the least non-negative integer such that

d

q( G − T ) 6  |T | +  d

for every  T ⊆ V .

( ∗)

Then there exists a set  T  for which equality holds in ( ∗): this follows from the minimality of  d  if  d >  0, and from  q( G − ∅) >  |∅| + 0 if  d = 0.

Let  S  be such a set  T  of maximum cardinality, and let  C :=  CG−S.

S, C

We first show that every component  C ∈ C  is odd. If  |C|  is even, pick a vertex  c ∈ C, and let  S0 :=  S ∪ { c }  and  C0 :=  C − c. Then  C0  has odd order, and thus has at least one odd component. Hence,  q( G − S0) > q( G − S) + 1. Since  T :=  S  satisfies ( ∗) with equality, we obtain q( G − S0) >  q( G − S) + 1 =  |S| +  d + 1 =  |S0| +  d >  q( G − S0) ( ∗)

with equality, which contradicts the maximality of  S.

Next we prove the assertion (ii), that every  C ∈ C  is factor-critical.

Suppose there exist  C ∈ C  and  c ∈ C  such that  C0 :=  C − c  has no 1-factor. By the induction hypothesis (and the fact that, as shown earlier, for fixed  G  our theorem implies Tutte’s theorem) there exists a set T 0 ⊆ V ( C0) with

q( C0 − T 0)  > |T 0| .

Since  |C|  is odd and hence  |C0|  is even, the numbers  q( C0 − T 0) and  |T 0|

are either both even or both odd, so they cannot differ by exactly 1. We may therefore sharpen the above inequality to

q( C0 − T 0) >  |T 0| + 2  .

For  T :=  S ∪ { c } ∪ T 0  we thus obtain

q( G − T ) =  q( G − S)  −  1 +  q( C0 − T 0)

>  |S| +  d −  1 +  |T 0| + 2

=  |T | +  d

>  q( G − T )

( ∗)

with equality, again contradicting the maximality of  S.

It remains to show that  S  is matchable to  G − S. If  S =  ∅, this is trivial, so we assume that  S 6=  ∅. Since  T :=  S  satisfies ( ∗) with

38

2. Matching

equality, this implies that  C  too is non-empty. We now apply Corollary

H

2.1.3 to  H :=  HS, but ‘backwards’, i.e. with  A :=  C. Given  C0 ⊆ C, set  S0 :=  NH ( C0)  ⊆ S. Since every  C ∈ C0  is an odd component also of G − S0, we have

|NH( C0) | =  |S0| >  q( G − S0)  − d >  |C0| − d .

( ∗)

By Corollary 2.1.3, then,  H  contains a matching of cardinality

|C| − d =  q( G − S)  − d =  |S| ,

which is therefore a matching of  S.

¤

S

Let us consider once more the set  S  from Theorem 2.2.3, together C

with any matching  M  in  G. As before, we write  C :=  CG−S. Let us denote by  kS  the number of edges in  M  with at least one end in  S, and kS, kC

by  kC  the number of edges in  M  with both ends in  G − S. Since each C ∈ C  is odd, at least one of its vertices is not incident with an edge of the second type. Therefore every matching  M  satisfies

³

´

kS  6  |S|  and  kC  6 1  |V | − |S| − |C| .

(1)

2

M 0

Moreover,  G  contains a matching  M 0 with equality in both cases: first S

choose  |S|  edges between  S  and

C  according to (i), and then use (ii) to

¡

¢

find a suitable set of 1  |C| −  1 edges in every component  C ∈ C. This 2

matching  M 0 thus has exactly

³

´

|M 0 | =  |S| + 1  |V | − |S| − |C|

(2)

2

edges.

Now (1) and (2) together imply that  every  matching  M  of maximum cardinality satisfies both parts of (1) with equality: by  |M | >  |M 0 |

¡

¢

and (2),  M  has at least  |S| + 1  |V | − |S| − |C|  edges, which implies by 2

(1) that neither of the inequalities in (1) can be strict. But equality in (1), in turn, implies that  M  has the structure described above: by kS =  |S|, every vertex  s ∈ S  is the end of an edge  st ∈ M  with  t ∈ G − S,

¡

¢

¢

and by  kC = 1  |V | − |S| − |C|  exactly 1 ( |C| −  1 edges of  M  lie in  C, 2

2

for every  C ∈ C. Finally, since these latter edges miss only one vertex in each  C, the ends  t  of the edges  st  above lie in different components  C

for different  s.

The seemingly technical Theorem 2.2.3 thus hides a wealth of structural information: it contains the essence of a detailed description of all maximum-cardinality matchings in all graphs.2

2 A reference to the full statement of this structural result, known as the  Gallai-Edmonds matching theorem, is given in the notes at the end of this chapter.

2.3 Path covers

39

2.3 Path covers

Let us return for a moment to König’s duality theorem for bipartite

graphs, Theorem 2.1.1. If we orient every edge of  G  from  A  to  B, the theorem tells us how many disjoint directed paths we need in order to

cover all the vertices of  G: every directed path has length 0 or 1, and clearly the number of paths in such a ‘path cover’ is smallest when it

contains as many paths of length 1 as possible—in other words, when it

contains a maximum-cardinality matching.

In this section we put the above question more generally: how many

paths in a given directed graph will suffice to cover its entire vertex set?

Of course, this could be asked just as well for undirected graphs. As it turns out, however, the result we shall prove is rather more trivial in the undirected case (exercise), and the directed case will also have an interesting corollary.

A  directed path  is a directed graph  P 6=  ∅  with distinct vertices x 0 , . . . , xk  and edges  e 0 , . . . , ek− 1 such that  ei  is an edge directed from xi  to  xi+1, for all  i < k. We denote the last vertex  xk  of  P  by ter( P ).

ter( P )

In this section,  path  will always mean ‘directed path’. A  path cover  of a path

directed graph  G  is a set of disjoint paths in  G  which together contain path cover

all the vertices of  G. Let us denote the maximum cardinality of an independent set of vertices in  G  by  α( G).

α( G)

Theorem 2.3.1. (Gallai & Milgram 1960)

Every directed graph G has a path cover by at most α( G)  paths.

Proof . Given two path covers  P 1 , P 2 of a graph, we write  P 1  < P 2 if P 1  < P 2

{  ter( P )  | P ∈ P 1  } ⊆ {  ter( P )  | P ∈ P 2  }  and  |P 1 | < |P 2 |. We shall prove the following:

If P is a <-minimal path cover of G, then G contains an

independent set { vP | P ∈ P } of vertices with vP ∈ P for

( ∗)

every P ∈ P.

Clearly, ( ∗) implies the assertion of the theorem.

We prove ( ∗) by induction on  |G|. Let  P =  { P 1 , . . . , Pm }  be given P, Pi, m

as in ( ∗), and let  vi := ter( Pi) for every  i. If  { vi |  1 6  i  6  m }  is vi

independent, there is nothing more to show; we may therefore assume

that  G  has an edge from  v 2 to  v 1. Since  P 2 v 2 v 1 is again a path, the minimality of  P  implies that  v 1 is not the only vertex of  P 1; let  v  be v

the vertex preceding  v 1 on  P 1. Then  P0 :=  { P 1 v, P 2 , . . . , Pm }  is a path P0

cover of  G0 :=  G − v 1 (Fig. 2.3.1). We first show that  P0  is  < -minimal G0

with this property.

Suppose that  P00 < P0  is another path cover of  G0. If a path  P ∈ P00

ends in  v, we may replace  P  in  P00  by  P vv 1 to obtain a smaller path cover of  G  than  P, a contradiction to the minimality of  P. If a path

40

2. Matching

v 1

v 2

v

. . .

P 1

P 2

Pm

Fig. 2.3.1.  The path cover  P0  of  G0

P ∈ P00  ends in  v 2 (but none in  v), we replace  P  in  P00  by  P v 2 v 1, again contradicting the minimality of  P. Hence  {  ter( P )  | P ∈ P00 } ⊆

{ v 3 , . . . , vm }, and in particular  |P00|  6  |P| −  2. But now  P00  and the trivial path  { v 1  }  together form a path cover of  G  that contradicts the minimality of  P.

Hence  P0  is minimal, as claimed. By the induction hypothesis,

{ V ( P )  | P ∈ P0 }  has an independent set of representatives. But this is also a set of representatives for  P, and ( ∗) is proved.

¤

As a corollary to Theorem 2.3.1 we now deduce a classic result from the theory of partial orders. Recall that a subset of a partially ordered chain

set ( P,  6) is a  chain  in  P  if its elements are pairwise comparable; it is antichain

an  antichain  if they are pairwise incomparable.

Corollary 2.3.2. (Dilworth 1950)

In every finite partially ordered set ( P,  6) , the minimum number of chains covering P is equal to the maximum cardinality of an antichain in P .

Proof . If  A  is an antichain in  P  of maximum cardinality, then clearly P  cannot be covered by fewer than  |A|  chains. The fact that  |A|  chains will suffice follows from Theorem 2.3.1 applied to the directed graph on P  with the edge set  { ( x, y)  | x < y }.

¤

Exercises

1.

Let  M  be a matching in a bipartite graph  G. Show that if  M  is sub-optimal, i.e. contains fewer edges than some other matching in  G, then G  contains an augmenting path with respect to  M . Does this fact generalize to matchings in non-bipartite graphs?

(Hint. Symmetric difference.)

Exercises

41

2.

Describe an algorithm that finds, as efficiently as possible, a matching of maximum cardinality in any bipartite graph.

3.

Find an infinite counterexample to the statement of the marriage the-

orem.

4.

Let  k  be an integer. Show that any two partitions of a finite set into k-sets admit a common choice of representatives.

5.

Let  A  be a finite set with subsets  A 1 , . . . , An, and let  d 1 , . . . , dn ∈  N.

Show that there are disjoint subsets  Dk ⊆ Ak, with  |Dk| =  dk  for all k  6  n, if and only if

¯¯ [ ¯ X

¯

¯

Ai¯ >

di

i∈I

i∈I

for all  I ⊆ {  1 , . . . , n }.

¡

¢

6.+ Prove  Sperner’s lemma: in an  n-set  X  there are never more than n

bn/ 2 c

subsets such that none of these contains another.

¡

¢

(Hint. Construct

n

b

chains covering the power set lattice of  X.)

n/ 2 c

7.

Find a set  S  for Theorem 2.2.3 when  G  is a forest.

8.

Using (only) Theorem 2.2.3, show that a  k-connected graph with at least 2 k  vertices contains a matching of size  k. Is this best possible?

9.

A graph  G  is called (vertex-)  transitive  if, for any two vertices  v, w ∈ G, there is an automorphism of  G  mapping  v  to  w. Using the observations following the proof of Theorem 2.2.3, show that every transitive connected graph is either factor-critical or contains a 1-factor.

(Hint. Consider the cases of  S =  ∅  and  S 6=  ∅  separately.) 10.

Show that a graph  G  contains  k  independent edges if and only if q( G − S) 6  |S| +  |G| −  2 k  for all sets  S ⊆ V ( G).

(Hint. For the ‘if’ direction, suppose that  G  has no  k  independent edges, and apply Tutte’s 1-factor theorem to the graph  G ∗ K|G|− 2 k.

Alternatively, use Theorem 2.2.3.)

11.  −  Find a cubic graph without a 1-factor.

12.

Derive the marriage theorem from Tutte’s theorem.

13.  −  Prove the undirected version of the theorem of Gallai & Milgram (without using the directed version).

14.

Derive the marriage theorem from the theorem of Gallai & Milgram.

15.  −  Show that a partially ordered set of at least  rs + 1 elements contains either a chain of size  r + 1 or an antichain of size  s + 1.

16.

Prove the following dual version of Dilworth’s theorem: in every finite partially ordered set ( P,  6), the minimum number of antichains covering  P  is equal to the maximum cardinality of a chain in  P .

17.

Derive K¨

onig’s theorem from Dilworth’s theorem.

42

2. Matching

18.+ Find a partially ordered set that has no infinite antichain but cannot be covered by finitely many chains.

(Hint. N  ×  N.)

Notes

There is a very readable and comprehensive monograph about matching in

finite graphs: L. Lovász & M.D. Plummer,  Matching Theory, Annals of Discrete Math. 29, North Holland 1986. All the references for the results in this chapter can be found there.

As we shall see in Chapter 3, K¨

onig’s Theorem of 1931 is no more than the

bipartite case of a more general theorem due to Menger, of 1929. At the time, neither of these results was nearly as well known as Hall’s marriage theorem,

which was proved even later, in 1935. To this day, Hall’s theorem remains one of the most applied graph-theoretic results. Its special case that both partition sets have the same size was proved implicitly already by Frobenius (1917) in a paper on determinants.

Our proof of Tutte’s 1-factor theorem is based on a proof by Lovász (1975). Our extension of Tutte’s theorem, Theorem 2.2.3 (including the informal discussion following it) is a lean version of a comprehensive structure theorem for matchings, due to Gallai (1964) and Edmonds (1965). See Lovász & Plummer for a detailed statement and discussion of this theorem.

Theorem 2.3.1 is due to T. Gallai & A.N. Milgram, Verallgemeinerung eines graphentheoretischen Satzes von Rédei,  Acta Sci. Math. (Szeged) 21

(1960), 181–186.

3

Connectivity

Our definition of  k-connectedness, given in Chapter 1.4, is somewhat unintuitive. It does not tell us much about ‘connections’ in a  k-connected graph: all it says is that we need at least  k  vertices to  dis connect it. The following definition—which, incidentally, implies the one above—might

have been more descriptive: ‘a graph is  k- connected  if any two of its vertices can be joined by  k  independent paths’.

It is one of the classic results of graph theory that these two defini-

tions are in fact equivalent, are dual aspects of the same property. We shall study this theorem of Menger (1927) in some depth in Section 3.3.

In Sections 3.1 and 3.2, we investigate the structure of the 2-con-

nected and the 3-connected graphs. For these small values of  k  it is still possible to give a simple general description of how these graphs can be constructed.

In the remaining sections of this chapter we look at other concepts of

connectedness, more recent than the standard one but no less important: the number of  H-paths in a graph for a given subgraph  H; the number of edge-disjoint spanning trees; and the existence of disjoint paths linking up several given pairs of vertices.

3.1 2-Connected graphs and subgraphs

A maximal connected subgraph without a cutvertex is called a  block .

block

Thus, every block of a graph  G  is either a maximal 2-connected subgraph, or a bridge (with its ends), or an isolated vertex. Conversely, every such subgraph is a block. By their maximality, different blocks of  G  overlap in at most one vertex, which is then a cutvertex of  G. Hence, every edge of  G  lies in a unique block, and  G  is the union of its blocks.

In a sense, blocks are the 2-connected analogues of components, the

maximal connected subgraphs of a graph. While the structure of  G  is

44

3. Connectivity

determined fully by that of its components, however, it is not captured completely by the structure of its blocks: since the blocks need not be disjoint, the way they intersect defines another structure, giving a coarse picture of  G  as if viewed from a distance.

The following proposition describes this coarse structure of  G  as formed by its blocks. Let  A  denote the set of cutvertices of  G, and  B

the set of its blocks. We then have a natural bipartite graph on  A ∪ B

block

formed by the edges  aB  with  a ∈ B. This  block graph  of  G  is shown in graph

Figure 3.1.1.

B0

B0

a0

a0

B

B

a

a

Fig. 3.1.1.  A graph and its block graph

Proposition 3.1.1.  The block graph of a connected graph is a tree.

¤

Proposition 3.1.1 reduces the structure of a given graph to that of its blocks. So what can we say about the blocks themselves? The following

proposition gives a simple method by which, in principle, a list of all 2-connected graphs could be compiled:

[ 4.2.5 ]

Proposition 3.1.2.  A graph is 2-connected if and only if it can be constructed from a cycle by successively adding H-paths to graphs H

already constructed (Fig. 3.1.2).

Fig. 3.1.2.  The construction of 2-connected graphs

3.1 2-Connected graphs and subgraphs

45

Proof . Clearly, every graph constructed as described is 2-connected.

Conversely, let a 2-connected graph  G  be given. Then  G  contains a cycle, and hence has a maximal subgraph  H  constructible as above.

H

Since any edge  xy ∈ E( G) r  E( H) with  x, y ∈ H  would define an  H-

path,  H  is an induced subgraph of  G. Thus if  H 6=  G, then by the connectedness of  G  there is an edge  vw  with  v ∈ G − H  and  w ∈ H. As G  is 2-connected,  G − w  contains a  v– H  path  P . Then  wvP  is an  H-path in  G, and  H ∪ wvP  is a constructible subgraph of  G  larger than  H. This contradicts the maximality of  H.

¤

3.2 The structure of 3-connected graphs

We start this section with the analogue of Proposition 3.1.2 for 3-connectedness: our first theorem describes how every 3-connected graph

can be obtained from a  K 4 by a succession of elementary operations preserving 3-connectedness. We then prove a deep result of Tutte about

the algebraic structure of the cycle space of 3-connected graphs; this will play an important role again in Chapter 4.5.

Lemma 3.2.1.  If G is 3-connected and |G| >  4 , then G has an edge e

[ 4.4.3 ]

such that G/e is again 3-connected.

Proof . Suppose there is no such edge  e. Then, for every edge  xy ∈ G, xy

the graph  G/xy  contains a separating set  S  of at most 2 vertices. Since κ( G) > 3, the contracted vertex  vxy  of  G/xy (see Chapter 1.7) lies in  S  and  |S| = 2, i.e.  G  has a vertex  z /

∈ { x, y }  such that  { vxy, z }

z

separates  G/xy. Then any two vertices separated by  { vxy, z }  in  G/xy are separated in  G  by  T :=  { x, y, z }. Since no proper subset of  T

separates  G, every vertex in  T  has a neighbour in every component  C

C

of  G − T .

We choose the edge  xy, the vertex  z, and the component  C  so that

|C|  is as small as possible, and pick a neighbour  v  of  z  in  C (Fig. 3.2.1).

v

x

y

z

v

C

T

Fig. 3.2.1.  Separating vertices in the proof of Lemma 3.2.1

46

3. Connectivity

By assumption,  G/zv  is again not 3-connected, so again there is a vertex w

w  such that  { z, v, w }  separates  G, and as before every vertex in  { z, v, w }

has a neighbour in every component of  G − { z, v, w }.

As  x  and  y  are adjacent,  G −{ z, v, w }  has a component  D  such that D ∩ { x, y } =  ∅. Then every neighbour of  v  in  D  lies in  C (since  v ∈ C), so  D ∩ C 6=  ∅  and hence  D $  C  by the choice of  D. This contradicts the choice of  xy,  z  and  C.

¤

Theorem 3.2.2. (Tutte 1961)

A graph G is 3-connected if and only if there exists a sequence G 0 , . . . , Gn of graphs with the following properties:

(i)  G 0 =  K 4  and Gn =  G;

(ii)  Gi+1  has an edge xy with d( x) , d( y) > 3  and Gi =  Gi+1 /xy, for every i < n.

Proof . If  G  is 3-connected, a sequence as in the theorem exists by Lemma

3.2.1. Note that all the graphs in this sequence are 3-connected.

Conversely, let  G 0 , . . . , Gn  be a sequence of graphs as stated; we xy

show that if  Gi =  Gi+1 /xy  is 3-connected then so is  Gi+1, for every  i < n.

S

Suppose not, let  S  be a separating set of at most 2 vertices in  Gi+1, and C 1 , C 2

let  C 1 , C 2 be two components of  Gi+1  − S. As  x  and  y  are adjacent, we may assume that  { x, y } ∩ V ( C 1) =  ∅ (Fig. 3.2.2). Then  C 2 contains nei-y

x

C 1

S

C 2

Fig. 3.2.2.  The position of  xy ∈ Gi+1 in the proof of Theorem 3.2.2

ther both vertices  x, y  nor a vertex  v /

∈ { x, y }: otherwise  vxy  or  v  would

be separated from  C 1 in  Gi  by at most two vertices, a contradiction.

But now  C 2 contains only one vertex: either  x  or  y. This contradicts our assumption of  d( x) , d( y) > 3.

¤

Theorem 3.2.2 is the essential core of a result of Tutte known as his wheel theorem.1 Like Proposition 3.1.2 for 2-connected graphs, it enables us to construct all 3-connected graphs by a simple inductive process

depending only on local information: starting with  K 4, we pick a vertex v  in a graph constructed already, split it into two adjacent vertices  v0, v00, and join these to the former neighbours of  v  as we please—provided only that  v0  and  v00  each acquire at least 3 incident edges, and that every former neighbour of  v  becomes adjacent to at least one of  v0, v00.

1

wheel

Graphs of the form  Cn ∗ K 1 are called  wheels; thus,  K 4 is the smallest wheel.

3.2 The structure of 3-connected graphs

47

Theorem 3.2.3. (Tutte 1963)

The cycle space of a 3-connected graph is generated by its non-separating

[ 4.5.2 ]

induced cycles.

Proof . We apply induction on the order of the graph  G  considered.

(1.9.1)

In  K 4, every cycle is a triangle or (in terms of edges) the symmetric difference of triangles. As these are both induced and non-separating,

the assertion holds for  |G| = 4.

For the induction step, let  e =  xy  be an edge of  G  for which e =  xy

G0 :=  G/e  is again 3-connected; cf. Lemma 3.2.1. Then every edge G0

e0 ∈ E( G0) r  E( G) is of the form  e0 =  uve, where at least one of the two edges  ux  and  uy  lies in  G. We pick one that does (either  ux  or  uy), and identify it notationally with the edge  e0; thus  e0  now denotes both the edge  uve  of  G0  and one of the two edges  ux, uy. In this way we may regard  E( G0) as a subset of  E( G), and  E( G0) as a subspace of  E( G); thus all vector operations will take place unambiguously in  E( G).

Let us consider an induced cycle  C ⊆ G. If  e ∈ C  and  C =  C 3, we  fundamental call  C  a  fundamental triangle; then  C/e =  K 2. If  e ∈ C  but  C 6=  C 3, triangle

then  C/e  is a cycle in  G0. Finally if  e /

∈ C, then at most one of  x, y

lies on  C (otherwise  e  would be a chord), so the vertices of  C  in order also form a cycle in  G0  if we replace  x  or  y  by  ve; this cycle, too, will be denoted by  C/e. Thus, as long as  C  is not a fundamental triangle, C/e  will always denote a unique cycle in  G0. Note, however, that in the C/e

case of  e /

∈ C  the edge set of  C/e  when viewed as a subset of  E( G) need not coincide with  E( C), or even be a cycle at all; an example is shown in Figure 3.2.3.

x

x

u

w

e0

e00

e00

u

w

u

w

ve

e0

y

y

C

C/e

E( C/e)  ⊆ G

Fig. 3.2.3.  One of the four possibilities for  E( C/e) when  e /

∈ C

Let us refer to the non-separating induced cycles in  G  or  G0  as  basic basic cycles cycles. An element of  C( G) will be called  good  if it is a linear combination good

of basic cycles in  G; we thus want to show that every element of  C( G) is good. The basic idea of our proof is to contract a given cycle  C ∈ C( G) to  C/e, generate  C/e  in  C( G0) by induction, and try to lift the generators back to basic cycles in  G  that generate  C.

We start by proving three auxiliary facts.

Every fundamental triangle is a basic cycle in G.

(1)

48

3. Connectivity

A fundamental triangle,  wxyw  say, is clearly induced in  G. If it separated  G, then  { ve, w }  would separate  G0, which contradicts the choice of  e. This proves (1).

If C ⊆ G is an induced cycle but not a fundamental trian-

(2)

gle, then C +  C/e +  D ∈ { ∅, {e} } for some good D ∈ C( G) .

The gist of (2) is that, in terms of ‘generatability’,  C  and  C/e  differ only a little: after the addition of a permissible error term  D, at most in the edge  e. In which other edges, then, can  C  and  C/e  differ? Clearly at most in the two edges  eu =  uve  and  ew =  vew  incident with  ve  in  C/e; cf. Fig. 3.2.3. But these differences between the edge sets of  C/e  and C  are levelled out precisely by adding the corresponding fundamental triangles  uxy  and  xyw (which are basic by (1)). Indeed, let  Du  denote the triangle  uxy  if  eu /

∈ C  and  ∅  otherwise, and let  Dw  denote  xyw  if ew /

∈ C  and  ∅  otherwise. Then  D :=  Du +  Dw  satisfies (2) as desired.

Next, we show how to lift basic cycles of  G0  back to  G: For every basic cycle C0 ⊆ G0 there exists a basic cycle

(3)

C =  C( C0)  ⊆ G with C/e =  C0.

If  ve /

∈ C0, then (3) is satisfied with  C :=  C0. So we assume that  ve ∈ C0.

u, w

Let  u  and  w  be the two neighbours of  ve  on  C0, and let  P  be the  u– w P

path in  C0  avoiding  ve (Fig. 3.2.4). Then  P ⊆ G.

x

u

w

y

P

Fig. 3.2.4.  The search for a basic cycle  C  with  C/e =  C0

We first assume that  { ux, uy, wx, wy } ⊆ E( G), and consider (as Cx, Cy

candidates for  C) the cycles  Cx :=  uP wxu  and  Cy :=  uP wyu. Both are induced cycles in  G (because  C0  is induced in  G0), and clearly  Cx/e =

Cy/e =  C0. Moreover, neither of these cycles separates two vertices of  G − ( V ( P )  ∪ { x, y }) in  G, since  C0  does not separate such vertices in  G0. Thus, if  Cx (say) is a separating cycle in  G, then one of the components of  G − Cx  consists just of  y. Likewise, if  Cy  separates  G

then one of the arising components contains only  x. However, this cannot happen for both  Cx  and  Cy  at once: otherwise  NG( { x, y })  ⊆ V ( P ) and hence  NG( { x, y }) =  { u, w } (since  C0  has no chord), which contradicts κ( G) > 3. Hence, at least one of  Cx,  Cy  is a basic cycle in  G.

3.2 The structure of 3-connected graphs

49

It remains to consider the case that  { ux, uy, wx, wy } 6⊆ E( G), say ux /

∈ E( G). Then, as above, either  uP wyu  or  uP wxyu  is a basic cycle in  G, according as  wy  is an edge of  G  or not. This completes the proof of (3).

We now come to the main part of our proof, the proof that every

C ∈ C( G) is good. By Proposition 1.9.1 we may assume that  C  is an C

induced cycle in  G. By (1) we may further assume that  C  is not a fundamental triangle; so  C/e  is a cycle. Our aim is to argue as follows.

By (2),  C  differs from  C/e  at most by some good error term  D (and possibly in  e); by (3), the basic cycles  C0  of  G0  that sum to  C/e  by i

induction can be contracted from basic cycles of  G, which likewise differ from the  C0  only by a good error term  D

i

i (and possibly in  e); hence these

basic cycles of  G  and all the error terms together sum to  C—except that the edge  e  will need some special attention.

By the induction hypothesis,  C/e  has a representation

C/e =  C0 1 +  . . . +  C0k

C0 , . . . , C0

1

k

in  C( G0), where every  C0  is a basic cycle in  G0. For each  i, we obtain from i

(3) a basic cycle  C( C0)  ⊆ G  with  C( C0) /e =  C0 (in particular,  C( C0) is i

i

i

i

not a fundamental triangle), and from (2) some good  Di ∈ C( G) such that

C( C0i) +  C0i +  Di ∈ { ∅, {e} } .

(4)

We let

Ci :=  C( C0i) +  Di ;

C 1 , . . . , Ck

then  Ci  is good, and by (4) it differs from  C0  at most in  e. Again by (2), i

we have

C +  C/e +  D ∈ { ∅, {e} }

for some good  D ∈ C( G), i.e.  C +  D  differs from  C/e  at most in  e. But D

then  C +  D +  C 1 +  . . . +  Ck  differs from  C/e +  C0 1 +  . . . +  C0 =  ∅  at most k

in  e, that is,

C +  D +  C 1 +  . . . +  Ck ∈ { ∅, {e} } .

Since  C +  D +  C 1 +  . . . +  Ck ∈ C( G) but  {e} /

∈ C( G), this means that in

fact

C +  D +  C 1 +  . . . +  Ck =  ∅ , so  C =  D +  C 1 +  . . . +  Ck  is good.

¤

50

3. Connectivity

3.3 Menger’s theorem

The following theorem is one of the cornerstones of graph theory.

[ 3.6.2 ]

Theorem 3.3.1. (Menger 1927)

[ 8.1.2 ]

[ 12.3.9 ]

Let G = ( V, E)  be a graph and A, B ⊆ V . Then the minimum number

[ 12.4.4 ]

of vertices separating A from B in G is equal to the maximum number

[ 12.4.5 ]

of disjoint A–B paths in G.

We offer three proofs. Whenever  G, A, B  are given as in the theorem, k

we denote by  k =  k ( G, A, B) the minimum number of vertices separating A  from  B  in  G. Clearly,  G  cannot contain more than  k  disjoint  A– B

paths; our task will be to show that  k  such paths exist.

First proof. We prove the following stronger statement:

If P is any set of fewer than k disjoint A–B paths in G

then there is a set Q of |P| + 1  disjoint A–B paths whose

set of ends includes the set of ends of the paths in P.

Keeping  G  and  A  fixed, we let  B  vary and apply induction on  |G − B|.

Let  R  be an  A– B  path that avoids the (fewer than  k) vertices of  B  that lie on a path in  P. If  R  avoids all the paths in  P, then  Q :=  P ∪ { R }

is as desired. (This will happen for  |G − B| = 0 when all  A– B  paths are trivial.) If not, let  x  be the last vertex of  R  that lies on some  P ∈ P

¡

¢

(Fig. 3.3.1). Put  B0 :=  B ∪ V ( xP ∪ xR) and  P0 :=  P  r  { P } ∪ { P x }.

Then  |P0| =  |P|  and  k( G, A, B0) >  k( G, A, B), so by the induction hypothesis there is a set  Q0  of  |P| + 1 disjoint  A– B0  paths whose ends include those of the paths in  P0. Then  Q0  contains a path  Q  ending in  x, and a unique path  Q0  whose last vertex  y  is not among the last vertices of the paths in  P0. If  y /

∈ xP , we let  Q  be obtained from  Q0  by adding xP  to  Q, and adding  yR  to  Q0  if  y /

∈ B. Otherwise  y ∈ ˚

xP , and we let

Q  be obtained from  Q0  by adding  xR  to  Q  and adding  yP  to  Q0.

¤

R

P

x

P

x

P

A

R

x

B

Fig. 3.3.1.  Paths in the first proof of Menger’s theorem

3.3 Menger’s theorem

51

Second proof. We show by induction on  |G| +  kGk  that  G  contains  k disjoint  A– B  paths. For all  G, A, B  with  k ∈ {  0 ,  1  }  this is true. For the induction step let  G, A, B  with  k > 2 be given, and assume that the assertion holds for graphs with fewer vertices or edges.

If there is a vertex  x ∈ A ∩ B, then  G − x  contains  k −  1 disjoint  A– B

paths by the induction hypothesis. (Why?) Together with the trivial

path  { x }, these form the desired paths in  G. We shall therefore assume that

A ∩ B =  ∅ .

(1)

We first construct the desired paths for the case that  A  and  B  are separated by a set  X ⊆ V  with  |X| =  k  and  X 6=  A, B. Let  CA  be X

the union of all the components of  G − X  meeting  A; note that  CA 6=  ∅, since  |A| >  k =  |X|  but  A 6=  X. The subgraph  CB  defined likewise is not CA, CB

empty either, and  CA ∩ CB =  ∅. Let us write  GA :=  G [  V ( CA)  ∪ X ] and GB :=  G [  V ( CB)  ∪ X ]. Since every  A– B  path in  G  contains an  A– X

GA, GB

path in  GA, we cannot separate  A  from  X  in  GA  by fewer than  k  vertices.

Thus, by the induction hypothesis,  GA  contains  k  disjoint  A– X  paths (Fig. 3.3.2). In the same way, there are  k  disjoint  X– B  paths in  GB. As

|X| =  k, we can put these paths together to form  k  disjoint  A– B  paths.

A

A

B

GA

GB

X

Fig. 3.3.2.  Disjoint  A– X  paths in  GA

For the general case, let  P  be any  A– B  path in  G. By (1),  P  has an edge  ab  with  a /

∈ B  and  b /

∈ A. Let  Y  be a set of as few vertices as

ab

possible separating  A  from  B  in  G − ab (Fig. 3.3.3). Then  Ya :=  Y ∪ { a }

Y

and  Yb :=  Y ∪ { b }  both separate  A  from  B  in  G, and by definition of  k Ya, Yb

we have

|Ya|, |Yb| >  k .

If equality holds here, we may assume by the case already treated that

{ Ya, Yb } ⊆ { A, B }, so  { Ya, Yb } =  { A, B }  since  a /∈ B  and  b /∈ A. Thus, Y =  A ∩ B. Since  |Y | >  k −  1 > 1, this contradicts (1).

We therefore have either  |Ya| > k  or  |Yb| > k, and hence  |Y | >  k.

By the induction hypothesis, then, there are  k  disjoint  A– B  paths even in  G − ab ⊆ G.

¤

52

3. Connectivity

a

b

P

A

B

Y

Fig. 3.3.3.  Separating  A  from  B  in  G − ab Applied to a bipartite graph, Menger’s theorem specializes to the

assertion of K¨

onig’s theorem (2.1.1). For our third proof, we now adapt the alternating path proof of K¨

onig’s theorem to the more general set-

P

up of Theorem 3.3.1. Let again  G, A, B  be given, and let  P  be a set of disjoint  A– B  paths in  G. We write

[

V [  P ] :=

{ V ( P )  | P ∈ P }

[

E [  P ] :=

{ E( P )  | P ∈ P } .

A walk  W =  x 0 e 0 x 1 e 1  . . . en− 1 xn  in  G  with  ei 6=  ej  for  i 6=  j  is said to be  alternating  with respect to  P  if the following three conditions are alternating

satisfied for all  i < n (Fig. 3.3.4):

walk

(i) if  ei =  e ∈ E [  P ], then  W  traverses the edge  e  backwards, i.e.

xi+1  ∈ P˚

xi  for some  P ∈ P;

(ii) if  xi =  xj  with  i 6=  j, then  xi ∈ V [  P ]; (iii) if  xi ∈ V [  P ], then  { ei− 1 , ei } ∩ E [  P ]  6=  ∅.2

W

xn

x

P

0

B

A

Fig. 3.3.4.  An alternating walk from  A  to  B

2 For  i = 0 we let  { ei− 1 , ei } :=  { e 0  }.

3.3 Menger’s theorem

53

Let us consider a walk  W =  x 0 e 0 x 1 e 1  . . . en− 1 xn  from  A  r  V [  P ]

W, xi, ei

to  B  r  V [  P ], alternating with respect to  P. By (ii), any vertex outside V [  P ] occurs at most once on  W . Since the edges  ei  of  W  are all distinct, (iii) implies that any vertex in  V [  P ] occurs at most twice on  W . This can happen in two ways: if  xi =  xj  with 0  < i < j < n, say, then either ei− 1 , ej ∈ E [  P ] and  ei, ej− 1  /

∈ E [  P ]

or ei, ej− 1  ∈ E [  P ] and  ei− 1 , ej /

∈ E [  P ]  .

Lemma 3.3.2.  If such a walk W exists, then G contains |P| + 1  disjoint A–B paths.

Proof . Let  H  be the graph on  V [  P ]  ∪{ x 0 , . . . , xn }  whose edge set is the symmetric difference of  E [  P ] with  { e 0 , . . . , en− 1  }. In  H, the ends of the paths in  P  and of  W  have degree 1 (or 0, if the path or  W  is trivial), and all other vertices have degree 0 or 2. For each of the  |P| + 1 vertices a ∈ ( A ∩ V [  P ])  ∪ { x 0  }, therefore, the component of  H  containing  a  is a path,  P =  v 0  . . . vk  say, which starts in  a  and ends in  A  or  B. Using P

conditions (i) and (iii), one easily shows by induction on  i = 0 , . . . , k −  1

that  P  traverses each of its edges  e =  vivi+1 in the forward direction with respect to  P  or  W . (Formally: if  e ∈ P 0  with  P 0 ∈ P, then  vi ∈ P 0˚

vi+1;

if  e =  ej ∈ W , then  vi =  xj  and  vi+1 =  xj+1.) Hence,  P  ends in  B. As we have  |P| + 1 disjoint such paths  P , this completes the proof.

¤

Third proof of Menger’s theorem. Let  P  be a set of as many disjoint P

A– B  paths in  G  as possible. Unless otherwise stated, all alternating walks considered are alternating with respect to  P. We set

A 1 :=  A ∩ V [  P ] and  A 2 :=  A  r  A 1  , A 1 , A 2

and

B 1 :=  B ∩ V [  P ] and  B 2 :=  B  r  B 1  .

B 1 , B 2

For every path  P ∈ P, let  xP  be the last vertex of  P  that lies on xP

some alternating walk starting in  A 2; if no such vertex exists, let  xP  be the first vertex of  P . Clearly, the set

X :=  { xP | P ∈ P }

X

has cardinality  |P|; it thus suffices to show that  X  separates  A  from  B.

Let  Q  be any  A– B  path in  G; we show that  Q  meets  X. Suppose Q

not. By the maximality of  P, the path  Q  meets  V [  P ]. Since the  A–

V [  P ] path in  Q  is trivially an alternating walk,  Q  also meets the vertex set  V [  P0 ] of

P0 :=  { P xP | P ∈ P } ;

P0

54

3. Connectivity

y, P

let  y  be the last vertex of  Q  in  V [  P0 ], let  P  be the path in  P  containing  y, x, W

and let  x :=  xP . Finally, let  W  be an alternating walk from  A 2 to  x, as in the definition of  xP . By assumption,  Q  avoids  X  and hence  x, so  y ∈ P˚

x, and  W ∪ xP yQ  is a walk from  A 2 to  B (Fig. 3.3.5). If this walk is alternating and ends in  B 2, we are home: then  G  contains

|P| + 1 disjoint  A– B  paths by Lemma 3.3.2, contrary to the maximality of  P.

P

x

y

Q

z

yQ

W

Fig. 3.3.5.  Alternating walks in the third proof of Menger’s theorem

How could  W ∪ xP yQ  fail to be an alternating walk? For a start, W  might already use an edge of  xP y. But if  x0  is the first vertex of  W

x0, W 0

on  xP˚

y, then  W 0 :=  W x0P y  is an alternating walk from  A 2 to  y. (By W x0  we mean the initial segment of  W  ending at the first occurrence of x0  on  W ; from there onwards,  W 0  follows  P  back to  y.) Even our new walk  W 0yQ  need not yet be alternating:  W 0  might still meet ˚

yQ. By

definition of  P0  and  W , however, and the choice of  y  on  Q, we have V ( W 0)  ∩ V [  P ]  ⊆ V [  P0 ] and  V (˚

yQ)  ∩ V [  P0 ] =  ∅ .

Thus,  W 0  and ˚

yQ  can meet only outside  P.

z

If  W 0  does indeed meet ˚

yQ, let  z  be the first vertex of  W 0  on ˚

yQ. As

z  lies outside  V [  P ], it occurs only once on  W 0 (condition (ii)), and we let W 00

W 00 :=  W 0zQ. On the other hand if  W 0 ∩˚

yQ =  ∅, we set  W 00 :=  W 0 ∪ yQ.

In both cases,  W 00  is alternating with respect to  P0, because  W 0  is and ˚

yQ

avoids  V [  P0 ]. (Note that  W 00  satisfies condition (iii) at  y  in the second case, while in the first case (iii) is not applicable to  z.) By definition of  P0, therefore,  W 00  avoids  V [  P ] r  V [  P0 ]; in particular,  V (˚

yQ)  ∩ V [  P ] =  ∅.

Thus  W 00  is also alternating with respect to  P, and it ends in  B 2. (Note that  y  cannot be the last vertex of  W 00, since  y ∈ P˚

x  and hence  y /

∈ B.)

Furthermore,  W 00  starts in  A 2, because  W  does. We may therefore use  W 00  with Lemma 3.3.2 to obtain the desired contradiction to the maximality of  P.

¤

3.3 Menger’s theorem

55

A set of  a– B  paths is called an  a– B fan  if any two of the paths have fan

only  a  in common.

Corollary 3.3.3.  For B ⊆ V and a ∈ V  r  B, the minimum number of

[ 10.1.2 ]

vertices 6=  a separating a from B in G is equal to the maximum number of paths forming an a–B fan in G.

Proof . Apply Theorem 3.3.1 with  A :=  N ( a).

¤

Corollary 3.3.4.  Let a and b be two distinct vertices of G.

(i)  If ab /

∈ E, then the minimum number of vertices 6=  a, b separating a from b in G is equal to the maximum number of independent

a–b paths in G.

(ii)  The minimum number of edges separating a from b in G is equal

to the maximum number of edge-disjoint a–b paths in G.

Proof . (i) Apply Theorem 3.3.1 with  A :=  N ( a) and  B :=  N ( b).

(ii) Apply Theorem 3.3.1 to the line graph of  G, with  A :=  E( a) and  B :=  E( b).

¤

[ 4.2.10 ]

Theorem 3.3.5. (Global Version of Menger’s Theorem)

[ 6.6.1 ]

[ 9.4.2 ]

(i)  A graph is k-connected if and only if it contains k independent paths between any two vertices.

(ii)  A graph is k-edge-connected if and only if it contains k edge-

disjoint paths between any two vertices.

Proof . (i) If a graph  G  contains  k  independent paths between any two vertices, then  |G| > k  and  G  cannot be separated by fewer than  k  vertices; thus,  G  is  k-connected.

Conversely, suppose that  G  is  k-connected (and, in particular, has more than  k  vertices) but contains vertices  a, b  not linked by  k  indepen-a, b

dent paths. By Corollary 3.3.4 (i),  a  and  b  are adjacent; let  G0 :=  G − ab.

G0

Then  G0  contains at most  k −  2 independent  a– b  paths. By Corollary

3.3.4 (i), we can separate  a  and  b  in  G0  by a set  X  of at most  k −  2

X

vertices. As  |G| > k, there is at least one further vertex  v /

∈ X ∪ { a, b }

v

in  G. Now  X  separates  v  in  G0  from either  a  or  b—say, from  a. But then  X ∪ { b }  is a set of at most  k −  1 vertices separating  v  from  a  in  G, contradicting the  k-connectedness of  G.

(ii) follows straight from Corollary 3.3.4 (ii).

¤

56

3. Connectivity

3.4 Mader’s theorem

In analogy to Menger’s theorem we may consider the following question: given a graph  G  with an induced subgraph  H, up to how many independent  H-paths can we find in  G?

In this section, we present without proof a deep theorem of Mader,

which solves the above problem in a fashion similar to Menger’s theorem.

Again, the theorem says that an upper bound on the number of such

paths that arises naturally from the size of certain separators is indeed attained by some suitable set of paths.

X

What could such an upper bound look like? Clearly, if  X ⊆ V ( G − H) F

and  F ⊆ E( G − H) are such that every  H-path in  G  has a vertex or an edge in  X ∪ F , then  G  cannot contain more than  |X ∪ F |  independent H-paths. Hence, the least cardinality of such a set  X ∪ F  is a natural upper bound for the maximum number of independent  H-paths. (Note that every  H-path meets  G − H, because  H  is induced in  G  and edges of  H  do not count as  H-paths.)

In contrast to Menger’s theorem, this bound can still be improved.

Clearly, we may assume that no edge in  F  has an end in  X: otherwise this edge would not be needed in the separator. Let  Y :=  V ( G − H) r  X, CF

and denote by  CF  the set of components of the graph ( Y, F ). Since every H-path avoiding  X  contains an edge from  F , it has at least two vertices

∂C

in  ∂C  for some  C ∈ CF , where  ∂C  denotes the set of vertices in  C  with a neighbour in  G − X − C (Fig. 3.4.1). The number of independent C

CF

∂C

H

X

Fig. 3.4.1.  An  H-path in  G − X

H-paths in  G  is therefore bounded above by

³

X ¥

¦´

MG( H)

M

1

G( H ) := min

|X| +

|∂C| ,

2

C ∈CF

X

where the minimum is taken over all  X  and  F  as described above:  X ⊆

V ( G − H) and  F ⊆ E( G − H − X) such that every  H-path in  G  has a vertex or an edge in  X ∪ F .

3.4 Mader’s theorem

57

Now Mader’s theorem says that this upper bound is always attained

by some set of independent  H-paths:

Theorem 3.4.1. (Mader 1978)

Given a graph G with an induced subgraph H, there are always MG( H) independent H-paths in G.

In order to obtain direct analogues to the vertex and edge version

of Menger’s theorem, let us consider the two special cases of the above problem where either  F  or  X  is required to be empty. Given an induced subgraph  H ⊆ G, we denote by  κG( H) the least cardinality of a vertex κG( H)

set  X ⊆ V ( G − H) that meets every  H-path in  G. Similarly, we let λG( H) denote the least cardinality of an edge set  F ⊆ E( G) that meets λG( H)

every  H-path in  G.

Corollary 3.4.2.  Given a graph G with an induced subgraph H, there are at least  1  κ

λ

2

G( H )  independent H -paths and at least  1

2

G( H )  edge-

disjoint H-paths in G.

Proof .

To prove the first assertion, let  k  be the maximum num-

k

ber of independent  H-paths in  G. By Theorem 3.4.1, there are sets X ⊆ V ( G − H) and  F ⊆ E( G − H − X) with X ¥

¦

k =  |X| +

1  |∂C|

2

C ∈CF

such that every  H-path in  G  has a vertex in  X  or an edge in  F . For every C ∈ CF  with  ∂C 6=  ∅, pick a vertex  v ∈ ∂C  and let  YC :=  ∂C  r  { v }; if

¥

¦

∂C =  ∅, let  Y

1

C :=  ∅. Then

|∂C| > 1  |Y

2

2

C |  for all  C ∈ CF . Moreover,

S

for  Y :=

Y

C ∈C

C  every  H -path has a vertex in  X ∪ Y . Hence

Y

F

X

k >  |X| +

1  |Y

|X ∪ Y | > 1 κ

2

C | > 1

2

2

G( H )

C ∈CF

as claimed.

The second assertion follows from the first by considering the line

graph of  G (Exercise 16).

¤

It may come as a surprise to see that the bounds in Corollary 3.4.2

are best possible (as general bounds): one can find examples for  G  and H  where  G  contains no more than 1  κ

2

G( H ) independent  H -paths or no

more than 1  λ

2

G( H ) edge-disjoint  H -paths (Exercises 17 and 18).

58

3. Connectivity

3.5 Edge-disjoint spanning trees

The edge version of Menger’s theorem tells us when a graph  G  contains  k edge-disjoint paths between any two vertices. The actual routes of these paths within  G  may depend a lot on the choice of those two vertices: having found the paths for one pair of endvertices, we are not necessarily better placed to find them for another pair.

In a situation where quick access to a set of  k  edge-disjoint paths between any two vertices is desirable, it may be a good idea to ask for more than just  k-edge-connectedness. For example, if  G  has  k edge-disjoint spanning trees, there will be  k  canonical such paths between any two vertices, one in each tree.

When do such trees exist? If they do, the graph is clearly  k-edge-connected. The converse is easily seen to be false; indeed, it is not

even clear whether any edge-connectivity, however large, will imply the existence of  k  edge-disjoint spanning trees. Our first aim in this section will be to study conditions under which  k  edge-disjoint spanning trees exist.

As before, it is easy to write down some obvious necessary conditions

for the existence of  k  edge-disjoint spanning trees. With respect to any partition of  V ( G) into  r  sets, every spanning tree of  G  has at least  r −  1

cross-edges

cross-edges, edges whose ends lie in different partition sets (why?). Thus if  G  has  k  edge-disjoint spanning trees, it has at least  k ( r −  1) cross-edges.

Once more, this obvious necessary condition is also sufficient:

Theorem 3.5.1. (Tutte 1961; Nash-Williams 1961)

A multigraph contains k edge-disjoint spanning trees if and only if for every partition P of its vertex set it has at least k ( |P | −  1)  cross-edges.

Before we prove Theorem 3.5.1, let us note a surprising corollary:

to ensure the existence of  k  edge-disjoint spanning trees, it suffices to raise the edge-connectivity to just 2 k:

[ 6.4.4 ]

Corollary 3.5.2.  Every  2 k-edge-connected multigraph G has k edge-disjoint spanning trees.

Proof . Every set in a vertex partition of  G  is joined to other partition sets by at least 2 k  edges. Hence, for any partition into  r  sets,  G  has P

at least 1

r

2 k =  kr  cross-edges. The assertion thus follows from

2

i=1

Theorem 3.5.1.

¤

G = ( V, E)

For the proof of Theorem 3.5.1, let a multigraph  G = ( V, E) and k, F

k ∈  N be given. Let  F  be the set of all  k-tuples  F = ( F 1 , . . . , Fk) of edge-disjoint spanning forests in  G  with the maximum total number of

3.5 Edge-disjoint spanning trees

59

¯

¯

edges, i.e. such that  kF k := ¯ E [  F ]¯ with  E [  F ] :=  E( F 1)  ∪ . . . ∪ E( Fk) E[ F ] , kF k

is as large as possible.

If  F = ( F 1 , . . . , Fk)  ∈ F  and  e ∈ E  r  E [  F ], then every  Fi +  e  contains a cycle ( i = 1 , . . . , k): otherwise we could replace  Fi  by  Fi +  e  in  F

and obtain a contradiction to the maximality of  kF k. Let us consider an edge  e0 6=  e  of this cycle, for some fixed  i. Putting  F 0 :=  F

i

i +  e − e0,

and  F 0 :=  F

) is again in  F;

j

j  for all  j 6=  i, we see that  F 0 := ( F 0

1 , . . . , F 0

k

edge

we say that  F 0  has been obtained from  F  by the  replacement  of the  replacement edge  e0  with  e. Note that the component of  Fi  containing  e0  keeps its vertex set when it changes into a component of  F 0. Hence for every path i

x . . . y ⊆ F 0  there is a unique path  xF

i

iy  in  Fi; this will be used later.

xFiy

We now consider a fixed  k-tuple  F  0 = ( F  0

1  , . . . , F  0)  ∈ F . The set

k

F  0

of all  k-tuples in  F  that can be obtained from  F  0 by a series of edge replacements will be denoted by  F 0. Finally, we let

F 0

[ ( E  r E [ F ])

E 0

E 0 :=

F ∈F 0

and  G 0 := ( V, E 0).

G 0

Lemma 3.5.3.  For every e 0  ∈ E  r  E [  F  0 ]  there exists a set U ⊆ V that is connected in every F  0  ( i = 1 , . . . , k) and contains the ends of e 0 .

i

Proof . As  F  0  ∈ F 0, we have  e 0  ∈ E 0; let  C 0 be the component of  G 0

C 0

containing  e 0. We shall prove the assertion for  U :=  V ( C 0).

U

Let  i ∈ {  1 , . . . , k }  be given; we have to show that  U  is connected i

in  F  0. To this end, we first prove the following:

i

Let F = ( F 1 , . . . , Fk)  ∈ F 0 , and let ( F 0 1 , . . . , F 0 )  have been k

obtained from F by the replacement of an edge of Fi. If

(1)

x, y are the ends of a path in F 0 ∩ C 0 , then also xF

i

iy ⊆ C  0 .

Let  e =  vw  be the new edge in  E( F 0) r  E [  F ]; this is the only edge of i

F 0  not lying in  F

y: otherwise we would have

i

i. We assume that  e ∈ xF 0

i

xFiy =  xF 0y  and nothing to show. It suffices to show that  vF

i

iw ⊆ C  0:

then ( xF 0y − e)  ∪ vF

i

iw  is a connected subgraph of  Fi ∩ C  0 that contains

x, y, and hence also  xFiy. Let  e0  be any edge of  vFiw. Since we could replace  e0  in  F ∈ F 0 by  e  and obtain an element of  F 0 not containing  e0, we have  e0 ∈ E 0. Thus  vFiw ⊆ G 0, and hence  vFiw ⊆ C 0 since v, w ∈ xF 0y ⊆ C 0. This proves (1).

i

In order to prove that  U =  V ( C 0) is connected in  F  0 we show that, i

for every edge  xy ∈ C 0, the path  xF  0 y  exists and lies in  C 0. As  C 0 is i

connected, the union of all these paths will then be a connected spanning subgraph of  F  0 [  U ].

i

So let  e =  xy ∈ C 0 be given. As  e ∈ E 0, there exist an  s ∈  N

and  k-tuples  F r = ( F r

1  , . . . , F r ) for  r = 1 , . . . , s  such that each  F r  is k

obtained from  F r− 1 by edge replacement and  e ∈ E  r  E [  F s ]. Setting

60

3. Connectivity

F :=  F s  in (1), we may think of  e  as a path of length 1 in  F 0 ∩ C 0.

i

Successive applications of (1) to  F =  F s, . . . , F  0 then give  xF  0 y ⊆ C 0

i

as desired.

¤

(1.5.3)

Proof of Theorem 3.5.1. We prove the backward implication by induction on  |G|. For  |G| = 2 the assertion holds. For the induction step, we now suppose that for every partition  P  of  V  there are at least k ( |P |−  1) cross-edges, and construct  k  edge-disjoint spanning trees in  G.

F  0

Pick a  k-tuple  F  0 = ( F  0

1  , . . . , F  0)  ∈ F . If every  F  0 is a tree, we are k

i

done. If not, we have

k

X

kF  0 k =

kF  0 k

i

< k ( |G| −  1)

i=1

by Corollary 1.5.3. On the other hand, we have  kGk >  k ( |G| −  1) by assumption: consider the partition of  V  into single vertices. So there e 0

exists an edge  e 0  ∈ E  r  E [  F  0 ]. By Lemma 3.5.3, there exists a set U

U ⊆ V  that is connected in every  F  0 and contains the ends of  e i

0; in

particular,  |U | > 2. Since every partition of the contracted multigraph G/U  induces a partition of  G  with the same cross-edges,3  G/U  has at least  k ( |P | −  1) cross-edges with respect to any partition  P . By the induction hypothesis, therefore,  G/U  has  k  edge-disjoint spanning trees T 1 , . . . , Tk. Replacing in each  Ti  the vertex  vU  contracted from  U  by the spanning tree  F  0  ∩ G [  U ] of  G [  U ], we obtain  k  edge-disjoint spanning i

trees in  G.

¤

graph

Let us say that subgraphs  G 1 , . . . , Gk  of a graph  G partition G  if partition

their edge sets form a partition of  E( G). Our spanning tree problem may then be recast as follows: into how  many connected  spanning subgraphs can we partition a given graph? The excuse for rephrasing our simple

tree problem in this more complicated way is that it now has an obvious dual (cf. Theorem 1.5.1): into how  few acyclic (spanning) subgraphs can we partition a given graph? Or for given  k: which graphs can be partitioned into at most  k  forests?

An obvious necessary condition now is that every set  U ⊆ V ( G) induces at most  k ( |U | −  1) edges, no more than  |U | −  1 for each forest.

Once more, this condition turns out to be sufficient too. And surprising-ly, this can be shown with the help of Lemma 3.5.3, which was designed for the proof of our theorem on edge-disjoint spanning trees:

Theorem 3.5.4. (Nash-Williams 1964)

A multigraph G = ( V, E)  can be partitioned into at most k forests if and only if kG [  U ] k  6  k ( |U | −  1)  for every non-empty set U ⊆ V .

3 see Chapter 1.10 on the contraction of multigraphs

3.5 Edge-disjoint spanning trees

61

Proof . The forward implication was shown above. Conversely, we show

(1.5.3)

that every  k-tuple  F = ( F 1 , . . . , Fk)  ∈ F  partitions  G, i.e. that  E [  F ] =

E. If not, let  e ∈ E  r  E [  F ]. By Lemma 3.5.3, there exists a set  U ⊆ V

that is connected in every  Fi  and contains the ends of  e. Then  G [  U ]

contains  |U | −  1 edges from each  Fi, and in addition the edge  e. Thus kG [  U ] k > k ( |U| −  1), contrary to our assumption.

¤

The least number of forests forming a partition of a graph  G  is called the  arboricity  of  G. By Theorem 3.5.4, the arboricity is a measure for arboricity

the maximum local density: a graph has small arboricity if and only if

it is ‘nowhere dense’, i.e. if and only if it has no subgraph  H  with  ε( H) large.

3.6 Paths between given pairs of vertices

A graph with at least 2 k  vertices is said to be  k-linked  if for every 2 k  dis-k-linked

tinct vertices  s 1 , . . . , sk,  t 1 , . . . , tk  it contains  k  disjoint paths  P 1 , . . . , Pk with  Pi =  si . . . ti  for all  i. Thus unlike in Menger’s theorem, we are not merely asking for  k  disjoint paths between two  sets  of vertices: we insist that each of these paths shall link a specified pair of endvertices.

Clearly, every  k-linked graph is  k-connected. The converse, however, is far from true: being  k-linked is generally a much stronger property than  k-connectedness. But still, the two properties are related: our aim in this section is to prove the existence of a function  f : N  →  N such that every  f ( k)-connected graph is  k-linked.

As a lemma, we need a result that would otherwise belong in Chap-

ter 8:

Theorem 3.6.1. (Mader 1967)

There is a function h: N  →  N  such that every graph with average degree at least h( r)  contains Kr as a topological minor, for every r ∈  N .

Proof . For  r  6 2, the assertion holds with  h( r) = 1; we now assume that

(1.2.2)

¡ ¢

(1.3.1)

r > 3. We show by induction on  m =  r, . . . , r  that every graph  G  with 2

average degree  d( G) > 2 m  has a topological minor  X  with  r  vertices and

¡ ¢

m  edges; for  m =  r  this implies the assertion with  h( r) = 2( r) 2 .

2

If  m =  r  then, by Propositions 1.2.2 and 1.3.1,  G  contains a cycle of length at least  ε( G) + 1 > 2 r− 1 + 1 >  r + 1, and the assertion follows with  X =  Cr.

¡ ¢

Now let  r < m  6  r , and assume the assertion holds for smaller  m.

2

Let  G  with  d( G) > 2 m  be given; thus,  ε( G) > 2 m− 1. Since  G  has a component  C  with  ε( C) >  ε( G), we may assume that  G  is connected.

Consider a maximal set  U ⊆ V ( G) such that  U  is connected in  G  and U

62

3. Connectivity

ε( G/U ) > 2 m− 1; such a set  U  exists, because  G  itself has the form  G/U

with  |U | = 1. Since  G  is connected, we have  N ( U )  6=  ∅.

H

Let  H :=  G [  N ( U ) ]. If  H  has a vertex  v  of degree  dH ( v)  <  2 m− 1, we may add it to  U  and obtain a contradiction to the maximality of  U : when we contract the edge  vvU  in  G/U , we lose one vertex and  dH ( v) + 1 6

2 m− 1 edges, so  ε  will still be at least 2 m− 1. Therefore  d( H) >  δ( H) > 2 m− 1. By the induction hypothesis,  H  contains a  T Y  with  |Y | =  r and  kY k =  m −  1. Let  x, y  be two branch vertices of this  T Y  that are non-adjacent in  Y . Since  x  and  y  lie in  N ( U ) and  U  is connected in  G, G  contains an  x– y  path whose inner vertices lie in  U . Adding this path to the  T Y , we obtain the desired  T X.

¤

How can Theorem 3.6.1 help with our aim to show that high connectivity will make a graph  k-linked? Since high connectivity forces the average degree up (even the minimum degree), we may assume by the

theorem that our graph contains a subdivision  K  of a large complete graph. Our plan now is to use Menger’s theorem to link the given vertices  si  and  ti  disjointly to branch vertices of  K, and then to join up the correct pairs of those branch vertices inside  K.

Theorem 3.6.2. (Jung 1970; Larman & Mani 1970)

There is a function f : N  →  N  such that every f ( k) -connected graph is k-linked, for all k ∈  N .

(3.3.1)

Proof . We prove the assertion for  f ( k) =  h(3 k) + 2 k, where  h  is a G

function as in Theorem 3.6.1. Let  G  be an  f ( k)-connected graph. Then K

d( G) >  δ( G) >  κ( G) >  h(3 k); choose  K =  T K 3 k ⊆ G  as in Theorem U

3.6.1, and let  U  denote its set of branch vertices.

si, ti

For the proof that  G  is  k-linked, let distinct vertices  s 1 , . . . , sk  and t 1 , . . . , tk  of  G  be given. By definition of  f ( k), we have  κ( G) > 2 k.

Hence by Menger’s theorem (3.3.1),  G  contains disjoint paths  P 1 , . . . , Pk, Pi, Qi

Q 1 , . . . , Qk, such that each  Pi  starts in  si, each  Qi  starts in  ti, and all P

these paths end in  U  but have no inner vertices in  U . Let the set  P  of these paths be chosen so that their total number of edges outside  E( K) is as small as possible.

ui

Let  u 1 , . . . , uk  be those  k  vertices in  U  that are not an end of a Li

path in  P. For each  i = 1 , . . . , k, let  Li  be the  U -path in  K (i.e., the vi

subdivided edge of the  K 3 k) from  ui  to the end of  Pi  in  U , and let  vi  be the first vertex of  Li  on any path  P ∈ P. By definition of  P,  P  has no more edges outside  E( K) than  P viLiui  does, so  viP =  viLi  and hence Mi

P =  Pi (Fig. 3.6.1). Similarly, if  Mi  denotes the  U -path in  K  from  ui wi

to the end of  Qi  in  U , and  wi  denotes the first vertex of  Mi  on any path in  P, then this path is  Qi. Then the paths  siPiviLiuiMiwiQiti  are disjoint for different  i  and show that  G  is  k-linked.

¤

3.6 Paths between given pairs of vertices

63

ui

si

w

P

i

vi

Qi

ti

P

L

i

i

Mi

Fig. 3.6.1.  Constructing an  si– ti  path via  ui In our proof of Theorem 3.6.2 we did not try to find any particularly good bound on the connectivity needed to force a graph to be  k-linked; the function  f  we used grows exponentially in  k. Not surprisingly, this is far from being best possible. It is still remarkable, though, that  f  can in fact be chosen linear: as Bollobás & Thomason (1996) have shown, every 22 k-connected graph is  k-linked.

Exercises

For the first three exercises, let  G  be a graph and  a, b ∈ V ( G). Suppose that X ⊆ V ( G) r  { a, b }  separates  a  from  b  in  G. We say that  X  separates  a  from  b minimally  if no proper subset of  X  separates  a  from  b  in  G.

1.  −  Show that  X  separates  a  from  b  minimally if and only if every vertex in  X  has a neighbour in the component  Ca  of  G − X  containing  a, and another in the component  Cb  of  G − X  containing  b.

2.

Let  X0 ⊆ V ( G) r  { a, b }  be another set separating  a  from  b, and define C0a  and  C0b  correspondingly. Show that both

Ya := ( X ∩ C0a)  ∪ ( X ∩ X0)  ∪ ( X0 ∩ Ca) and

Yb := ( X ∩ C0b)  ∪ ( X ∩ X0)  ∪ ( X0 ∩ Cb) separate  a  from  b (see figure).

X0

X

a

Ya

b

X

X0

3.

Do  Ya  and  Yb  separate  a  from  b  minimally if  X  and  X0  do? Are  |Ya|

and  |Yb|  minimal for vertex sets separating  a  from  b  if  |X|  and  |X0|  are?

4.

Let  X  and  X0  be minimal separating vertex sets in  G  such that  X

meets at least two components of  G − X0. Show that  X0  meets all the components of  G − X, and that  X  meets all the components of  G − X0.

64

3. Connectivity

5.  −  Prove the elementary properties of blocks mentioned at the beginning of Section 3.1.

6.

Show that the block graph of any connected graph is a tree.

7.

Show, without using Menger’s theorem, that any two vertices of a 2-

connected graph lie on a common cycle.

8.

For edges  e, e0 ∈ G  write  e ∼ e0  if either  e =  e0  or  e  and  e0  lie on some common cycle in  G. Show that  ∼  is an equivalence relation on  E( G) whose equivalence classes are the edge sets of the non-trivial blocks

of  G.

9.

Let  G  be a 2-connected graph but not a triangle, and let  e  be an edge of  G. Show that either  G − e  or  G/e  is again 2-connected.

10.

Let  G  be a 3-connected graph, and let  xy  be an edge of  G. Show that G/xy  is 3-connected if and only if  G − { x, y }  is 2-connected.

11.

(i) Show that every cubic 3-edge-connected graph is 3-connected.

(ii) Show that a graph is cubic and 3-connected if and only if it can

be constructed from a  K 4 by successive applications of the following operation: subdivide two edges by inserting a new vertex on each of

them, and join the two new subdividing vertices by an edge.

12.  −  Show that Menger’s theorem is equivalent to the following statement.

For every graph  G  and vertex sets  A, B ⊆ V ( G), there exist a set  P  of disjoint  A– B  paths in  G  and a set  X ⊆ V ( G) separating  A  from  B  in G  such that  X  has the form  X =  { xP | P ∈ P }  with  xP ∈ P  for all P ∈ P.

13.

Work out the details of the proof of Corollary 3.3.4 (ii).

14.

Let  k > 2. Show that every  k-connected graph of order at least 2 k contains a cycle of length at least 2 k.

15.

Let  k > 2. Show that in a  k-connected graph any  k  vertices lie on a common cycle.

16.

Derive the edge part of Corollary 3.4.2 from the vertex part.

(Hint. Consider the  H-paths in the graph obtained from the disjoint union of  H  and the line graph  L( G) by adding all the edges  he  such that  h  is a vertex of  H  and  e ∈ E( G) r  E( H) is an edge at  h.) 17.  −  To the disjoint union of the graph  H =  K 2 m+1 with  k  copies of  K 2 m+1

add edges joining  H  bijectively to each of the  K 2 m+1. Show that the resulting graph  G  contains at most  km = 1  κ

2

G( H ) independent  H -

paths.

18.

Find a bipartite graph  G, with partition classes  A  and  B  say, such that for  H :=  G [  A ] there are at most 1  λ

2

G( H ) edge-disjoint  H -paths in  G.

Exercises

65

19.+ Derive Tutte’s 1-factor theorem (2.2.1) from Mader’s theorem.

(Hint. Extend the given graph  G  to a graph  G0  by adding, for each vertex  v ∈ G, a new vertex  v0  and joining  v0  to  v. Choose  H ⊆ G0  so that the 1-factors in  G  correspond to the large enough sets of independent H-paths in  G0.)

20.

Find the error in the following short ‘proof’ of Theorem 3.5.1. Call a partition  non-trivial  if it has at least two classes and at least one of the classes has more than one element. We show by induction on  |V | +  |E|

that  G = ( V, E) has  k  edge-disjoint spanning trees if every non-trivial partition of  V  into  r  sets (say) has at least  k( r −  1) cross-edges. The induction starts trivially with  G =  K 1 if we allow  k  copies of  K 1 as a family of  k  edge-disjoint spanning trees of  K 1. We now consider the induction step. If every non-trivial partition of  V  into  r  sets (say) has more than  k( r −  1) cross-edges, we delete any edge of  G  and are done by induction. So  V  has a non-trivial partition  { V 1 , . . . , Vr }  with exactly k( r −  1) cross-edges. Assume that  |V 1 | > 2. If  G0 :=  G [  V 1 ] has  k disjoint spanning trees, we may combine these with  k  disjoint spanning trees that exist in  G/V 1 by induction. We may thus assume that  G0

has no  k  disjoint spanning trees. Then by induction it has a non-trivial vertex partition  { V 0 1 , . . . , V 0s }  with fewer than  k( s −  1) cross-edges.

Then  { V 0 1 , . . . , V 0s, V 2 , . . . , Vr }  is a non-trivial vertex partition of  G  into r +  s −  1 sets with fewer than  k( r −  1) +  k( s −  1) =  k(( r +  s −  1)  −  1) cross-edges, a contradiction.

21.  −  Show that every  k-linked graph is (2 k −  1)-connected.

Notes

Although connectivity theorems are doubtless among the most natural, and also the most applicable, results in graph theory, there is still no comprehensive monograph on this subject. Some areas are covered in B. Bollobás,  Extremal Graph Theory, Academic Press 1978, in R. Halin,  Graphentheorie, Wissenschaftliche Buchgesellschaft 1980, and in A. Frank’s chapter of the  Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995. A survey specifically of techniques and results on minimally  k-connected graphs (see below) is given by W. Mader, On vertices of degree  n  in minimally n-connected graphs and digraphs, in (D. Miklós, V.T. Sós & T. Sz˝

onyi, eds.)

Paul Erd˝

os is 80, Vol. 2, Proc. Colloq. Math. Soc. János Bolyai, Budapest 1996.

Our proof of Tutte’s Theorem 3.2.3 is due to C. Thomassen, Planarity and duality of finite and infinite graphs,  J. Combin. Theory B 29 (1980), 244–271.

This paper also contains Lemma 3.2.1 and its short proof from first principles.

(The lemma’s assertion, of course, follows from Tutte’s wheel theorem—its

significance lies in its independent proof, which has shortened the proofs of both of Tutte’s theorems considerably.)

An approach to the study of connectivity not touched upon in this chap-

ter is the investigation of  minimal k-connected graphs, those that lose their k-connectedness as soon as we delete an edge. Like all  k-connected graphs, these have minimum degree at least  k, and by a fundamental result of Halin

66

3. Connectivity

(1969), their minimum degree is exactly  k. The existence of a vertex of small degree can be particularly useful in induction proofs about  k-connected graphs.

Halin’s theorem was the starting point for a series of more and more sophisticated studies of minimal  k-connected graphs; see the books of Bollobás and Halin cited above, and in particular Mader’s survey.

Our first proof of Menger’s theorem is due to T. Böhme, F. Göring and J. Harant (manuscript 1999); the second to J.S. Pym, A proof of Menger’s theorem,  Monatshefte Math. 73 (1969), 81–88; the third to T. Grünwald (later Gallai), Ein neuer Beweis eines Mengerschen Satzes,  J. London Math. Soc. 13

(1938), 188–192. The global version of Menger’s theorem (Theorem 3.3.5) was first stated and proved by Whitney (1932).

Mader’s Theorem 3.4.1 is taken from W. Mader, Über die Maximalzahl kreuzungsfreier  H -Wege,  Arch. Math. 31 (1978), 387–402. The theorem may be viewed as a common generalization of Menger’s theorem and Tutte’s 1-

factor theorem (Exercise 19). Theorem 3.5.1 was proved independently by Nash-Williams and by Tutte; both papers are contained in  J. London Math.

Soc. 36 (1961). Theorem 3.5.4 is due to C.St.J.A. Nash-Williams, Decompositions of finite graphs into forests,  J. London Math. Soc. 39 (1964), 12. Our proofs follow an account by Mader (personal communication). Both results can be elegantly expressed and proved in the setting of matroids; see  §  18 in B. Bollobás,  Combinatorics, Cambridge University Press 1986.

In Chapter 8.1 we shall prove that, in order to force a topological  Kr  minor in a graph  G, we do not need an average degree of  G  as high as  h( r) = 2( r) 2

(as used in our proof of Theorem 3.6.1): the average degree required can be bounded above by a function quadratic in  r (Theorem 8.1.1). The improvement of Theorem 3.6.2 mentioned in the text is due to B. Bollobás & A.G. Thomason, Highly linked graphs,  Combinatorica 16 (1996), 313–320.

N. Robertson & P.D. Seymour, Graph Minors XIII: The disjoint paths problem,  J. Combin. Theory B 63 (1995), 65-110, showed that, for every fixed  k, there is an  O( n 3) algorithm that decides whether a given graph of order  n  is k-linked. If  k  is taken as part of the input, the problem becomes NP-hard.

4

Planar Graphs

When we draw a graph on a piece of paper, we naturally try to do this

as transparently as possible. One obvious way to limit the mess created by all the lines is to avoid intersections. For example, we may ask if we can draw the graph in such a way that no two edges meet in a point

other than a common end.

Graphs drawn in this way are called  plane graphs; abstract graphs that  can  be drawn in this way are called  planar . In this chapter we study both plane and planar graphs—as well as the relationship between

the two: the question of how an abstract graph might be drawn in

fundamentally different ways. After collecting together in Section 4.1 the few basic topological facts that will enable us later to prove all results rigorously without too much technical ado, we begin in Section 4.2 by

studying the structural properties of plane graphs. In Section 4.3, we

investigate how two drawings of the same graph can differ. The main

result of that section is that 3-connected planar graphs have essentially only one drawing, in some very strong and natural topological sense. The next two sections are devoted to the proofs of all the classical planarity criteria, conditions telling us when an abstract graph is planar. We

complete the chapter with a section on  plane duality, a notion with fascinating links to algebraic, colouring, and flow properties of graphs (Chapters 1.9 and 6.5).

The traditional notion of a graph drawing is that its vertices are

represented by points in the Euclidean plane, its edges are represented by curves between these points, and different curves meet only in common

endpoints. To avoid unnecessary topological complication, however, we

shall only consider curves that are piecewise linear; it is not difficult to show that any drawing can be straightened out in this way, so the two

notions come to the same thing.

68

4. Planar Graphs

4.1 Topological prerequisites

In this section we briefly review some basic topological definitions and facts needed later. All these facts have (by now) easy and well-known

proofs; see the notes for sources. Since those proofs contain no graph

theory, we do not repeat them here: indeed our aim is to collect precisely those topological facts that we need but do  not  want to prove. Later, all proofs will follow strictly from the definitions and facts stated here (and be guided by but not rely on geometric intuition), so the material presented now will help to keep elementary topological arguments in

those proofs to a minimum.

A  straight line segment  in the Euclidean plane is a subset of R2 that has the form  { p +  λ( q − p)  |  0 6  λ  6 1  }  for distinct points  p, q ∈  R2.

polygon

A  polygon  is a subset of R2 which is the union of finitely many straight line segments and is homeomorphic to the unit circle. Here, as later, any subset of a topological space is assumed to carry the subspace topology.

A  polygonal arc  is a subset of R2 which is the union of finitely many straight line segments and is homeomorphic to the closed unit interval

[ 0 ,  1 ]. The images of 0 and of 1 under such a homeomorphism are the endpoints  of this polygonal arc, which  links  them and runs  between  them.

arc

Instead of ‘polygonal arc’ we shall simply say  arc. If  P  is an arc between x  and  y, we denote the point set  P  r  { x, y }, the  interior  of  P , by ˚

P .

Let  O ⊆  R2 be an open set. Being linked by an arc in  O  defines an equivalence relation on  O. The corresponding equivalence classes are region

again open; they are the  regions  of  O. A closed set  X ⊆  R2 is said to separate

separate O  if  O  r  X  has more than one region. The  frontier  of a set frontier

X ⊆  R2 is the set  Y  of all points  y ∈  R2 such that every neighbourhood of  y  meets both  X  and R2 r  X. Note that if  X  is open then its frontier lies in R2 r  X.

The frontier of a region  O  of R2 r  X, where  X  is a finite union of points and arcs, has two important properties. The first is accessibility: if  x ∈ X  lies on the frontier of  O, then  x  can be linked to some point in  O

by a straight line segment whose interior lies wholly inside  O. As a consequence, any two points on the frontier of  O  can be linked by an arc whose interior lies in  O (why?). The second notable property of the frontier of O  is that it separates  O  from the rest of R2. Indeed, if  ϕ: [ 0 ,  1 ]  → P ⊆  R2

is continuous, with  ϕ(0)  ∈ O  and  ϕ(1)  /

∈ O, then  P  meets the frontier of

O  at least in the point  ϕ( y) for  y := inf  { x | ϕ( x)  /

∈ O }, the  first point

of  P  in R2 r  O.

[ 4.2.1 ]

[ 4.2.4 ]

[ 4.2.5 ]

Theorem 4.1.1. (Jordan Curve Theorem for Polygons)

[ 4.2.10 ]

For every polygon P ⊆  R2 , the set  R2 r  P has exactly two regions, of

[ 4.3.1 ]

[ 4.5.1 ]

which exactly one is bounded. Each of the two regions has the entire

[ 4.6.1 ]

polygon P as its frontier.

[ 5.1.2 ]

4.1 Topological prerequisites

69

With the help of Theorem 4.1.1, it is not difficult to prove the

following lemma.

[ 4.2.5 ]

Lemma 4.1.2.  Let P 1 , P 2 , P 3  be three arcs, between the same two end-

[ 4.2.6 ]

[ 4.2.10 ]

point but otherwise disjoint.

(i) R2 r ( P 1  ∪ P 2  ∪ P 3)  has exactly three regions, with frontiers P 1  ∪ P 2 , P 2  ∪ P 3  and P 1  ∪ P 3 .

(ii)  If P is an arc between a point in ˚

P 1  and a point in ˚

P 3  whose

interior lies in the region of  R2 r ( P 1  ∪ P 3)  that contains ˚

P 2 , then

˚

P ∩ ˚

P 2  6=  ∅.

P

P 2

P

P

3

1

Fig. 4.1.1.  The arcs in Lemma 4.1.2 (ii)

Our next lemma complements the Jordan curve theorem by saying

that an arc does  not  separate the plane. For easier application later, we phrase this a little more generally:

Lemma 4.1.3.  Let X 1 , X 2  ⊆  R2  be disjoint sets, each the union of

[ 4.2.1 ]

[ 4.2.3 ]

finitely many points and arcs, and let P be an arc between a point in X 1  and one in X 2  whose interior ˚

P lies in a region O of  R2 r ( X 1  ∪ X 2) .

Then O  r ˚

P is a region of  R2 r ( X 1  ∪ P ∪ X 2) .

P

O

X 1

X 2

Fig. 4.1.2. P  does not separate the region  O  of R2 r ( X 1  ∪ X 2) It remains to introduce a few terms and facts that will be used only

once, when we consider notions of equivalence for graph drawings in

Chapter 4.3.

As usual, we denote by  Sn  the  n-dimensional sphere, the set of Sn

points in R n+1 at distance 1 from the origin. The 2-sphere minus its

‘north pole’ (0 ,  0 ,  1) is homeomorphic to the plane; let us choose a fixed such homeomorphism  π:  S 2 r  { (0 ,  0 ,  1)  }→  R2 (for example, stereograph-π

ic projection). If  P ⊆  R2 is a polygon and  O  is the bounded region of

70

4. Planar Graphs

R2 r  P , let us call  C :=  π− 1( P ) a  circle on S 2, and the sets  π− 1( O) and S 2 r  π− 1( P ∪ O) the  regions  of  C.

Our last tool is the theorem of Jordan and Schoenflies, again adapt-

ed slightly for our purposes:

[ 4.3.1 ]

Theorem 4.1.4.  Let ϕ:  C 1  → C 2  be a homeomorphism between two circles on S 2 , let O 1  be a region of C 1 , and let O 2  be a region of C 2 .

Then ϕ can be extended to a homeomorphism C 1  ∪ O 1  → C 2  ∪ O 2 .

4.2 Plane graphs

plane

A  plane graph  is a pair ( V, E) of finite sets with the following properties graph

(the elements of  V  are again called  vertices, those of  E edges): (i)  V ⊆  R2;

(ii) every edge is an arc between two vertices;

(iii) different edges have different sets of endpoints;

(iv) the interior of an edge contains no vertex and no point of any

other edge.

A plane graph ( V, E) defines a graph  G  on  V  in a natural way. As long as no confusion can arise, we shall use the name  G  of this abstract graph S

also for the plane graph ( V, E), or for the point set  V ∪

E; similar

notational conventions will be used for abstract versus plane edges, for subgraphs, and so on.1

For every plane graph  G, the set R2 r  G  is open; its regions are the faces

faces  of  G. Since  G  is bounded—i.e., lies inside some sufficiently large disc  D—exactly one of its faces is unbounded: the face that contains R2 r  D. This face is the  outer face  of  G; the other faces are its  inner F ( G)

faces. We denote the set of faces of  G  by  F ( G). Note that if  H ⊆ G

then every face of  G  is contained in a face of  H.

In order to lay the foundations for the (easy but) rigorous introduc-

tion to plane graphs that this section aims to provide, let us descend

once now into the realm of truly elementary topology of the plane, and

prove what seems entirely obvious:2 that the frontier of a face of a plane graph  G  is always a subgraph of  G—not, say, half an edge. The following lemma states this formally, together with two similarly ‘obvious’

properties of plane graphs:

1 However, we shall continue to use r for differences of point sets and  −  for graph differences—which may help a little to keep the two apart.

2 Note that even the best intuition can only ever be ‘accurate’, i.e., coincide with what the technical definitions imply, inasmuch as those definitions do indeed formalize what is intuitively intended. Given the complexity of definitions in elementary topology, this can hardly be taken for granted.

4.2 Plane graphs

71

Lemma 4.2.1.  Let G be a plane graph and e an edge of G.

[ 4.5.1 ]

[ 4.5.2 ]

(i)  If X is the frontier of a face of G, then either e ⊆ X or X ∩˚

e =  ∅.

(ii)  If e lies on a cycle C ⊆ G, then e lies on the frontier of exactly two faces of G, and these are contained in distinct faces of C.

(iii)  If e lies on no cycle, then e lies on the frontier of exactly one face of G.

Proof . We prove all three assertions together. Let us start by considering

(4.1.1)

(4.1.3)

one point  x 0  ∈ ˚

e. We show that  x 0 lies on the frontier of either exactly two faces or exactly one, according as  e  lies on a cycle in  G  or not. We then show that every other point in ˚

e  lies on the frontier of exactly the

same faces as  x 0. Then the endpoints of  e  will also lie on the frontier of these faces—simply because every neighbourhood of an endpoint of  e  is also the neighbourhood of an inner point of  e.

G  is the union of finitely many straight line segments; we may assume that any two of these intersect in at most one point. Around every point  x ∈ ˚

e  we can find an open disc  Dx, with centre  x, which meets Dx

only those (one or two) straight line segments that contain  x.

Let us pick an inner point  x 0 from a straight line segment  S ⊆ e.

x 0 , S

Then  Dx ∩ G =  D ∩ S, so  D  r  G  is the union of two open half-discs.

0

x 0

x 0

Since these half-discs do not meet  G, they each lie in a face of  G. Let us denote these faces by  f 1 and  f 2; they are the only faces of  G  with  x 0

f 1 , f 2

on their frontier, and they may coincide (Fig. 4.2.1).

f

D

1

x 0

e

x 0

S

f 2

Fig. 4.2.1.  Faces  f 1 , f 2 of  G  in the proof of Lemma 4.2.1

If  e  lies on a cycle  C ⊆ G, then  Dx  meets both faces of  C (Theo-0

rem 4.1.1). The faces  f 1 , f 2 of  G  are therefore contained in distinct faces of  C—since  C ⊆ G, every face of  G  is a subset of a face of  C—and in particular  f 1  6=  f 2. If  e  does not lie on any cycle, then  e  is a bridge and thus links two disjoint point sets  X 1 , X 2 with  X 1  ∪ X 2 =  G  r˚

e. Clearly,

f 1  ∪˚

e ∪ f 2 is the subset of a face  f  of  G − e. (Why?) By Lemma 4.1.3,

f  r˚

e  is a face of  G. But  f  r˚

e  contains  f 1 and  f 2 by definition of  f , so f 1 =  f  r˚

e =  f 2 since  f 1,  f 2 and  f  are all faces of  G.

Now consider any other point  x 1  ∈ ˚

e. Let  P  be the arc from  x 0 to

x 1

x 1 contained in  e. Since  P  is compact, finitely many of the discs  Dx P

with  x ∈ P  cover  P . Let us enumerate these discs as  D 0 , . . . , Dn  in the  D 0 , . . . , Dn natural order of their centres along  P ; adding  Dx  or  D

as necessary,

0

x 1

we may assume that  D 0 =  Dx  and  D

. By induction on  n, one

0

n =  Dx 1

easily proves that every point  y ∈ Dn  r  e  can be linked by an arc inside y

72

4. Planar Graphs

z

( D 0  ∪ . . . ∪ Dn) r  e  to a point  z ∈ D 0 r  e (Fig. 4.2.2); then  y  and  z  are equivalent in R2 r  G. Hence, every point of  Dn  r  e  lies in  f 1 or in  f 2, so x 1 cannot lie on the frontier of any other face of  G. Since both half-discs of  D 0 r  e  can be linked to  Dn  r  e  in this way (swap the roles of  D 0

and  Dn), we find that  x 1 lies on the frontier of both  f 1 and  f 2.

¤

y

e

z

x 0

x 1

P

D 0

Dn

Fig. 4.2.2.  An arc from  y  to  D 0, close to  P

Corollary 4.2.2.  The frontier of a face is always the point set of a subgraph.

¤

The subgraph of  G  whose point set is the frontier of a face  f  is said to boundary

bound f  and is called its  boundary; we denote it by  G [  f ]. A face is G [  f ]

said to be  incident  with the vertices and edges of its boundary. Note that if  H ⊆ G  then every face  f  of  G  is contained in a face  f0  of  H. If G [  f ]  ⊆ H  then  f =  f 0 (why?); in particular,  f  is always also a face of its own boundary  G [  f ]. These basic facts will be used frequently in the proofs to come.

[ 4.6.1 ]

Proposition 4.2.3.  A plane forest has exactly one face.

(4.1.3)

Proof . Use induction on the number of edges and Lemma 4.1.3.

¤

With just one exception, different faces of a plane graph have dif-

ferent boundaries:

[ 4.3.1 ]

Lemma 4.2.4.  If a plane graph has different faces with the same boundary, then the graph is a cycle.

(4.1.1)

Proof . Let  G  be a plane graph, and let  H ⊆ G  be the boundary of distinct faces  f 1 , f 2 of  G. Since  f 1 and  f 2 are also faces of  H, Proposition 4.2.3 implies that  H  contains a cycle  C. By Lemma 4.2.1 (ii),  f 1 and  f 2

are contained in different faces of  C. Since  f 1 and  f 2 both have all of  H

as boundary, this implies that  H =  C: any further vertex or edge of  H

would lie in one of the faces of  C  and hence not on the boundary of the other. Thus,  f 1 and  f 2 are distinct faces of  C. As  C  has only two faces, it follows that  f 1  ∪ C ∪ f 2 = R2 and hence  G =  C.

¤

[ 4.3.1 ]

[ 4.4.3 ]

Proposition 4.2.5.  In a 2-connected plane graph, every face is bounded

[ 4.5.1 ]

[ 4.5.2 ]

by a cycle.

4.2 Plane graphs

73

Proof . Let  f  be a face in a 2-connected plane graph  G. We show by

(3.1.2)

(4.1.1)

induction on  |G|  that  G [  f ] is a cycle. If  G  is itself a cycle, this holds

(4.1.2)

by Theorem 4.1.1; we therefore assume that  G  is not a cycle.

By Proposition 3.1.2, there exist a 2-connected plane graph  H ⊆ G

H

and a plane  H-path  P  such that  G =  H ∪ P . The interior of  P  lies in a P

face  f 0  of  H, which by the induction hypothesis is bounded by a cycle  C.

f 0, C

If  f  is also a face of  H, we are home by the induction hypothesis.

If not, then the frontier of  f  meets  P  r  H, so  f ⊆ f 0. Therefore  f  is a face of  C ∪ P , and is hence bounded by a cycle (Lemma 4.1.2 (i)).

¤

maximal

A plane graph  G  is called  maximally plane, or just  maximal , if we  plane graph cannot add a new edge to form a plane graph  G0 %  G  with  V ( G0) =  V ( G).

We call  G  a  plane triangulation  if every face of  G (including the outer plane

triangulation

face) is bounded by a triangle.

Proposition 4.2.6.  A plane graph of order at least  3  is maximally plane

[ 4.4.1 ]

[ 5.4.2 ]

if and only if it is a plane triangulation.

Proof . Let  G  be a plane graph of order at least 3. It is easy to see that

(4.1.2)

if every face of  G  is bounded by a triangle, then  G  is maximally plane.

Indeed, any additional edge  e  would have its interior inside a face of  G

and its ends on the boundary of that face. Hence these ends are already adjacent in  G, so  G ∪ e  cannot satisfy condition (iii) in the definition of a plane graph.

Conversely, assume that  G  is maximally plane and let  f ∈ F ( G) be f

a face; let us write  H :=  G [  f ]. Since  G  is maximal as a plane graph, H

G [  H ] is complete: any two vertices of  H  that are not already adjacent in  G  could be linked by an arc through  f , extending  G  to a larger plane graph. Thus  G [  H ] =  Kn  for some  n—but we do not know yet which n

edges of  G [  H ] lie in  H.

Let us show first that  H  contains a cycle. If not, then  G  r  H 6=  ∅: by  G ⊇ Kn  if  n > 3, or else by  |G| > 3. On the other hand we have f ∪ H = R2 by Proposition 4.2.3 and hence  G =  H, a contradiction.

Since  H  contains a cycle, it suffices to show that  n  6 3: then  H =  K 3

as claimed. Suppose  n > 4, and let  C =  v 1 v 2 v 3 v 4 v 1 be a cycle in  G [  H ]

C, vi

(=  Kn). By  C ⊆ G, our face  f  is contained in a face  fC  of  C; let  f 0C

be the other face of  C. Since the vertices  v 1 and  v 3 lie on the boundary fC , f 0C

of  f , they can be linked by an arc whose interior lies in  fC  and avoids  G.

v

f 0

1

C

f

v

C ⊇ f

2

v 4

C

v 3

Fig. 4.2.3.  The edge  v 2 v 4 of  G  runs through the face  f 0C

74 4. Planar Graphs

Hence by Lemma 4.1.2 (ii), the plane edge  v 2 v 4 of  G [  H ] runs through f 0  rather than  f

C

C (Fig. 4.2.3).

Analogously, since  v 2 , v 4  ∈ G [  f ], the edge  v 1 v 3 runs through  f 0 . But the edges  v C

1 v 3 and  v 2 v 4 are disjoint, so

this contradicts Lemma 4.1.2 (ii). ¤

The following classic result of Euler (1752)—here stated in its sim-

plest form, for the plane—marks one of the common origins of graph

theory and topology. The theorem relates the number of vertices, edges

and faces in a plane graph: taken with the correct signs, these numbers always add up to 2. The general form of Euler’s theorem asserts the same for graphs suitably embedded in other surfaces, too: the sum obtained

is always a fixed number depending only on the surface, not on the

graph, and this number differs for distinct (orientable closed) surfaces.

Hence, any two such surfaces can be distinguished by a simple arithmetic invariant of the graphs embedded in them!3

Let us then prove Euler’s theorem in its simplest form:

Theorem 4.2.7. (Euler’s Formula)

Let G be a connected plane graph with n vertices, m edges, and ` faces.

Then

n − m +  ` = 2  .

(1.5.1)

Proof . We fix  n  and apply induction on  m. For  m  6  n −  1,  G  is a tree

(1.5.3)

and  m =  n −  1 (why?), so the assertion follows from Proposition 4.2.3.

e, G0

Now let  m >  n. Then  G  has an edge  e  lying on a cycle; let  G0 :=

f 1 , f 2

G − e. By Lemma 4.2.1 (ii),  e  lies on the boundary of exactly two faces f 1 ,  2

f 1 , f 2 of  G; we put  f 1 ,  2 :=  f 1  ∪˚

e ∪ f 2. We shall prove that

F ( G) r  { f 1 , f 2  } =  F ( G0) r  { f 1 ,  2  } , ψ ( ∗) without assuming that  f 1 ,  2  ∈ F ( G0). However, since ˚

e  must lie in some

face of  G0  and this will not be a face of  G, by ( ∗) it can only be  f 1 ,  2.

Thus again by ( ∗),  G0  has one face less than  G. As  G0  also has one edge less than  G, the assertion then follows from the induction hypothesis for  G0.

For our proof of ( ∗) we first consider any  f ∈ F ( G) r  { f 1 , f 2  }. By

Lemma 4.2.1 (i), we have  G [  f ]  ⊆ G  r˚

e =  G0. So  f  is also a face of  G0

(but obviously not equal to  f 1 ,  2) and hence lies in  F ( G0) r  { f 1 ,  2  }.

f 0

Conversely, let a face  f 0 6=  f 1 ,  2 of  G0  be given. Since  e  lies on the boundary of both  f 1 and  f 2, we can link any two points of  f 1 ,  2 by an f 0

arc in

1 ,  2

R2 r  G0, so  f 1 ,  2 lies inside a face  f0 1 ,  2 of  G0. Our assumption of  f 0 6=  f 1 ,  2 therefore implies  f 0 6⊆ f 1 ,  2 (as otherwise  f 0 ⊆ f 1 ,  2  ⊆ f 0 1 ,  2

3 This fundamental connection between graphs and surfaces lies at the heart of the proof of the famous Robertson-Seymour  graph minor theorem; see Chapter 12.5.

4.2 Plane graphs

75

and hence  f 0 =  f 1 ,  2 =  f 0 1 ,  2); let  x  be a point in  f0  r  f 1 ,  2. Then  x  lies x

in some face  f 6=  f 1 , f 2 of  G. As shown above,  f  is also a face of  G0.

f

Hence  x ∈ f ∩ f 0  implies  f =  f 0, and we have  f 0 ∈ F ( G) r  { f 1 , f 2  }  as desired.

¤

[ 4.4.1 ]

Corollary 4.2.8.  A plane graph with n > 3  vertices has at most  3 n −  6

[ 5.1.2 ]

[ 8.3.5 ]

edges. Every plane triangulation with n vertices has  3 n −  6  edges.

Proof . By Proposition 4.2.6 it suffices to prove the second assertion. In a plane triangulation  G, every face boundary contains exactly three edges, and every edge lies on the boundary of exactly two faces (Lemma 4.2.1).

The bipartite graph on  E( G)  ∪ F ( G) with edge set  { ef | e ⊆ G [  f ]  }  thus has exactly 2  |E( G) | = 3  |F ( G) |  edges. According to this identity we may replace  `  with 2 m/ 3 in Euler’s formula, and obtain  m = 3 n −  6.

¤

Euler’s formula can be useful for showing that certain graphs cannot occur as plane graphs. The graph  K 5, for example, has 10  >  3  ·  5  −  6

edges, more than allowed by Corollary 4.2.8. Similarly,  K 3 ,  3 cannot be a plane graph. For since  K 3 ,  3 is 2-connected but contains no triangle, every face of a plane  K 3 ,  3 would be bounded by a cycle of length > 4 (Proposition 4.2.5). As in the proof of Corollary 4.2.8 this implies 2 m > 4 `, which yields  m  6 2 n −  4 when substituted in Euler’s formula. But  K 3 ,  3

has 9  >  2  ·  6  −  4 edges.

Clearly, along with  K 5 and  K 3 ,  3 themselves, their subdivisions cannot occur as plane graphs either:

Corollary 4.2.9.  A plane graph contains neither K 5  nor K 3 ,  3  as a

[ 4.4.5 ]

[ 4.4.6 ]

topological minor.

¤

Surprisingly, it turns out that this simple property of plane graphs identifies them among all other graphs: as Section 4.4 will show, an arbitrary graph can be drawn in the plane if and only if it has no (topological)  K 5

or  K 3 ,  3 minor.

As we have seen, every face boundary in a 2-connected plane graph

is a cycle. In a 3-connected graph, these cycles can be identified combinatorially:

Proposition 4.2.10.  The face boundaries in a 3-connected plane graph

[ 4.3.2 ]

[ 4.5.2 ]

are precisely its non-separating induced cycles.

(3.3.5)

Proof . Let  G  be a 3-connected plane graph, and let  C ⊆ G. If  C  is a

(4.1.1)

(4.1.2)

non-separating induced cycle, then by the Jordan curve theorem its two faces cannot both contain points of  G  r  C. Therefore it bounds a face of  G.

Conversely, suppose that  C  bounds a face  f . By Proposition 4.2.5,

C, f

C  is a cycle. If  C  has a chord  e =  xy, then the components of  C − { x, y }

76

4. Planar Graphs

are linked by a  C-path in  G, because  G  is 3-connected. This path and e  both run through the other face of  C (not  f ) but do not intersect, a contradiction to Lemma 4.1.2 (ii).

It remains to show that  C  does not separate any two vertices  x, y ∈

G − C. By Menger’s theorem (3.3.5),  x  and  y  are linked in  G  by three independent paths. Clearly,  f  lies inside a face of their union, and by

Lemma 4.1.2 (i) this face is bounded by only two of the paths. The third therefore avoids  f  and its boundary  C.

¤

4.3 Drawings

planar

An embedding in the plane, or  planar embedding, of an (abstract) graph embedding

G  is an isomorphism between  G  and a plane graph ˜

G. The latter will

drawing

be called a  drawing  of  G. We shall not always distinguish notationally between the vertices and edges of  G  and of ˜

G.

In this section we investigate how two planar embeddings of a graph

can differ. For this to make sense, we first have to agree when two em-

beddings are to be considered the same: for example, if we compose one

embedding with a simple rotation of the plane, the resulting embedding

will hardly count as a genuinely different way of drawing that graph.

To prepare the ground, let us first consider three possible notions

of equivalence for plane graphs (refining abstract isomorphism), and see G;  V, E, F

how they are related. Let  G = ( V, E) and  G0 = ( V 0, E0) be two plane G0;  V 0, E0, F 0

graphs, with face sets  F ( G) =:  F  and  F ( G0) =:  F 0. Assume that  G  and G0  are isomorphic as abstract graphs, and let  σ:  V → V 0  be an isomor-σ

phism. Setting  xy 7→ σ( x) σ( y), we may extend  σ  in a natural way to a bijection  V ∪ E → V 0 ∪ E0  which maps  V  to  V 0  and  E  to  E0, and which preserves incidence (and non-incidence) between vertices and edges.

Our first notion of equivalence between plane graphs is perhaps

the most natural one. Intuitively, we would like to call our isomor-

phism  σ ‘topological’ if it is induced by a homeomorphism from the plane R2 to itself. To avoid having to grant the outer faces of  G  and G0  a special status, however, we take a detour via the homeomorphism π

π:  S 2 r  { (0 ,  0 ,  1)  } →  R2 chosen in Section 4.1: we call  σ  a  topological isomorphism  between the plane graphs  G  and  G0  if there exists a homeo-topological

morphism  ϕ:  S 2  → S 2 such that  ψ :=  π ◦ ϕ ◦ π− 1 induces  σ  on  V ∪ E.

isomorphism (More formally: we ask that  ψ  agree with  σ  on  V , and that it map every plane edge  e ∈ G  onto the plane edge  σ( e)  ∈ G0. Unless  ϕ  fixes the point (0 ,  0 ,  1), the map  ψ  will be undefined at  π( ϕ− 1(0 ,  0 ,  1)).) It can be shown that, up to topological isomorphism, inner and

outer faces are indeed no longer different: if we choose as  ϕ  a rotation of  S 2 mapping the  π− 1-image of a point of some inner face of  G  to the north pole (0 ,  0 ,  1) of  S 2, then  ψ  maps the rest of this face to the outer

4.3 Drawings

77

Fig. 4.3.1.  Two drawings of a graph that are not topologically isomorphic—why not?

face of  ψ( G). (To ensure that the edges of  ψ( G) are again piecewise linear, however, one may have to adjust  ϕ  a little.)

If  σ  is a topological isomorphism as above, then—except possibly for a pair of missing points where  ψ  or  ψ− 1 is undefined— ψ  maps the faces of  G  onto those of  G0 (proof?). In this way,  σ  extends naturally to a bijection  σ:  V ∪ E ∪ F → V 0 ∪ E0 ∪ F 0  which preserves incidence of vertices, edges and faces.

Let us single out this last property of a topological isomorphism

as the defining property for our second notion of equivalence for plane graphs: let us call our given isomorphism  σ  between the abstract graphs G  and  G0  a  combinatorial isomorphism  of the plane graphs  G  and  G0 combinatorial isomorphism

if it can be extended to a bijection  σ:  V ∪ E ∪ F → V 0 ∪ E0 ∪ F 0  that preserves incidence not only of vertices with edges but also of vertices and edges with faces. (Formally: we require that a vertex or edge  x ∈ G

shall lie on the boundary of a face  f ∈ F  if and only if  σ( x) lies on the boundary of the face  σ( f ).)

G0

G

Fig. 4.3.2.  Two drawings of a graph that are combinatorially

isomorphic but not topologically—why not?

If  σ  is a combinatorial isomorphism of the plane graphs  G  and  G0, it maps the face boundaries of  G  to those of  G0. Let us raise this property to our third definition of equivalence for plane graphs: we call our isomor-graph-

phism  σ  of the abstract graphs  G  and  G0  a  graph-theoretical isomorphism theoretical

isomorphism

of the plane graphs  G  and  G0  if

©

ª

©

ª

σ( G [  f ]) :  f ∈ F

=

G0 [  f 0 ] :  f 0 ∈ F 0

.

Thus, we no longer keep track of  which  face is bounded by a given subgraph: the only information we keep is whether a subgraph bounds

78

4. Planar Graphs

some face or not, and we require that  σ  map the subgraphs that do onto each other. At first glance, this third notion of equivalence may

appear a little less natural than the previous two. However, it has the practical advantage of being formally weaker and hence easier to verify, and moreover, it will turn out to be equivalent to the other two notions in most cases.

As we have seen, every topological isomorphism between two plane

graphs is also combinatorial, and every combinatorial isomorphism is also graph-theoretical. The following theorem shows that, for most graphs,

the converse is true as well:

Theorem 4.3.1.

(i)  Every graph-theoretical isomorphism between two plane graphs is combinatorial. Its extension to a face bijection is unique if and

only if the graph is not a cycle.

(ii)  Every combinatorial isomorphism between two 2-connected plane

graphs is topological.

(4.1.1)

(4.1.4)

Proof . Let  G = ( V, E) and  G0 = ( V 0, E0) be two plane graphs, put

(4.2.4)

(4.2.5)

F ( G) =:  F  and  F ( G0) =:  F 0, and let  σ:  V ∪ E → V 0 ∪ E0  be an isomorphism between the underlying abstract graphs.

(i) If  G  is a cycle, the assertion follows from the Jordan curve theorem. We now assume that  G  is not a cycle. Let  H  and  H0  be the sets of all face boundaries in  G  and  G0, respectively. If  σ  is a graph-theoretical isomorphism, then the map  H 7→ σ( H) is a bijection between  H  and  H0.

By Lemma 4.2.4, the map  f 7→ G [  f ] is a bijection between  F  and  H, and likewise for  F 0  and  H0. The composition of these three bijections is a bijection between  F  and  F 0, which we choose as  σ:  F → F 0. By construction, this extension of  σ  to  V ∪ E ∪ F  preserves incidences (and is unique with this property), so  σ  is indeed a combinatorial isomorphism.

σ

(ii) Let us assume that  G  is 2-connected, and that  σ  is a combinatorial isomorphism. We have to construct a homeomorphism  ϕ:  S 2  → S 2

which, for every vertex or plane edge  x ∈ G, maps  π− 1( x) to  π− 1( σ( x)).

˜

σ

Since  σ  is a combinatorial isomorphism, ˜

σ :  π− 1  ◦ σ ◦ π  is an incidence

preserving bijection from the vertices, edges and faces4 of ˜

G :=  π− 1( G)

˜

G, ˜

G0

to the vertices, edges and faces of ˜

G0 :=  π− 1( G0).

We construct  ϕ  in three steps. Let us first define  ϕ  on the vertex set of ˜

G, setting  ϕ( x) := ˜

σ( x) for all  x ∈ V ( ˜

G). This is trivially a

homeomorphism between  V ( ˜

G) and  V ( ˜

G0).

As the second step, we now extend  ϕ  to a homeomorphism between

˜

G  and ˜

G0  that induces ˜

σ  on  V ( ˜

G)  ∪ E( ˜

G). We may do this edge by

4 By the ‘vertices, edges and faces’ of ˜

G  and ˜

G0  we mean the images under  π− 1

of the vertices, edges and faces of  G  and  G0 (plus (0 ,  0 ,  1) in the case of the outer face). Their sets will be denoted by  V ( ˜

G),  E( ˜

G),  F ( ˜

G) and  V ( ˜

G0),  E( ˜

G0),  F ( ˜

G0),

and incidence is defined as inherited from  G  and  G0.

4.3 Drawings

79

˜

σ

S 2  ⊇ ˜

G

˜

  y

G0

⊇ S 2







π y



y π

R2  ⊇ G

G0

  y

⊇  R2

σ

Fig. 4.3.3.  Defining ˜

σ  via  σ

edge, as follows. Every edge  xy  of ˜

G  is homeomorphic to the edge

˜

σ( xy) =  ϕ( x) ϕ( y) of ˜

G0, by a homeomorphism mapping  x  to  ϕ( x) and y  to  ϕ( y). Then the union of all these homeomorphisms, one for every edge of ˜

G, is indeed a homeomorphism between ˜

G  and ˜

G0—our desired

extension of  ϕ  to ˜

G: all we have to check is continuity at the vertices

(where the edge homeomorphisms overlap), and this follows at once from

our assumption that the two graphs and their individual edges all carry the subspace topology in R3.

In the third step we now extend our homeomorphism  ϕ: ˜

G → ˜

G0  to

all of  S 2. This can be done analogously to the second step, face by face.

By Proposition 4.2.5, all face boundaries in ˜

G  and ˜

G0  are cycles. Now if

S

f  is a face of ˜

G  and  C  its boundary, then ˜

σ( C) :=

{ ˜ σ( e)  | e ∈ E( C)  }

bounds the face ˜

σ( f ) of ˜

G0. By Theorem 4.1.4, we may therefore extend the homeomorphism  ϕ:  C → ˜

σ( C) defined so far to a homeomorphism

from  C ∪ f  to ˜

σ( C)  ∪ ˜

σ( f ). We finally take the union of all these home-

omorphisms, one for every face  f  of ˜

G, as our desired homeomorphism

ϕ:  S 2  → S 2; as before, continuity is easily checked.

¤

So far, we have considered ways of comparing plane graphs. We

now come to our actual goal, the definition of equivalence for planar

embeddings. Let us call two planar embeddings  σ 1 , σ 2 of a graph  G

topologically (respectively,  combinatorially)  equivalent  if  σ 2  ◦ σ− 1 is a to-equivalent

1

embeddings

pological (respectively, combinatorial) isomorphism between  σ 1( G) and σ 2( G). If  G  is 2-connected, the two definitions coincide by Theorem

4.3.1, and we simply speak of  equivalent  embeddings. Clearly, this is indeed an equivalence relation on the set of planar embeddings of any

given graph.

Note that two drawings of  G  resulting from inequivalent embeddings may well be topologically isomorphic (exercise): for the equivalence of two embeddings we ask not only that some (topological or combinatorial) isomorphism exist between the their images, but that the canonical

isomorphism  σ 2  ◦ σ− 1 be a topological or combinatorial one.

1

Even in this strong sense, 3-connected graphs have only one embed-

ding up to equivalence:

Theorem 4.3.2. (Whitney 1932)

Any two planar embeddings of a 3-connected graph are equivalent.

80

4. Planar Graphs

(4.2.10)

Proof . Let  G  be a 3-connected graph with planar embeddings  σ 1:  G → G 1

and  σ 2:  G → G 2. By Theorem 4.3.1 it suffices to show that  σ 2  ◦ σ− 1 is 1

a graph-theoretical isomorphism, i.e. that  σ 1( C) bounds a face of  G 1 if and only if  σ 2( C) bounds a face of  G 2, for every subgraph  C ⊆ G. This follows at once from Proposition 4.2.10.

¤

4.4 Planar graphs: Kuratowski’s theorem

planar

A graph is called  planar  if it can be embedded in the plane: if it is isomorphic to a plane graph. A planar graph is  maximal , or  maximally planar , if it is planar but cannot be extended to a larger planar graph by adding an edge (but no vertex).

Drawings of maximal planar graphs are clearly maximally plane.

The converse, however, is not obvious: when we start to draw a planar

graph, could it happen that we get stuck half-way with a proper subgraph that is already maximally plane? Our first proposition says that this

can never happen, that is, a plane graph is never maximally plane just

because it is badly drawn:

Proposition 4.4.1.

(i)  Every maximal plane graph is maximally planar.

(ii)  A planar graph with n > 3  vertices is maximally planar if and only if it has  3 n −  6  edges.

(4.2.6)

Proof . Apply Proposition 4.2.6 and Corollary 4.2.8.

¤

(4.2.8)

Which graphs are planar? As we saw in Corollary 4.2.9, no planar graph contains  K 5 or  K 3 ,  3 as a topological minor. Our aim in this section is to prove the surprising converse, a classic theorem of Kuratowski: any graph without a topological  K 5 or  K 3 ,  3 minor is planar.

Before we prove Kuratowski’s theorem, let us note that it suffices

to consider ordinary minors rather than topological ones:

Proposition 4.4.2.  A graph contains K 5  or K 3 ,  3  as a minor if and only if it contains K 5  or K 3 ,  3  as a topological minor.

(1.7.2)

Proof . By Proposition 1.7.2 it suffices to show that every graph  G

with a  K 5 minor contains either  K 5 as a topological minor or  K 3 ,  3 as a minor. So suppose that  G <  K 5, and let  K ⊆ G  be minimal such that  K =  M K 5. Then every branch set of  K  induces a tree in  K, and between any two branch sets  K  has exactly one edge. If we take the tree induced by a branch set  Vx  and add to it the four edges joining it to other branch sets, we obtain another tree,  Tx  say. By the minimality

4.4 Planar graphs: Kuratowski’s theorem

81

Tx

Vx

Fig. 4.4.1.  Every  M K 5 contains a  T K 5 or  M K 3 ,  3

of  K,  Tx  has exactly 4 leaves, the 4 neighbours of  Vx  in other branch sets (Fig. 4.4.1).

If each of the five trees  Tx  is a  T K 1 ,  4 then  K  is a  T K 5, and we are done. If one of the  Tx  is not a  T K 1 ,  4 then it has exactly two vertices of degree 3. Contracting  Vx  onto these two vertices, and every other branch set to a single vertex, we obtain a graph on 6 vertices containing a  K 3 ,  3. Thus,  G <  K 3 ,  3 as desired.

¤

We first prove Kuratowski’s theorem for 3-connected graphs. This

is the heart of the proof: the general case will then follow easily.

Lemma 4.4.3.  Every 3-connected graph G without a K 5  or K 3 ,  3  minor is planar.

Proof . We apply induction on  |G|. For  |G| = 4 we have  G =  K 4, and

(3.2.1)

(4.2.5)

the assertion holds. Now let  |G| >  4, and assume the assertion is true for smaller graphs. By Lemma 3.2.1,  G  has an edge  xy  such that  G/xy xy

is again 3-connected. Since the minor relation is transitive,  G/xy  has no K 5 or  K 3 ,  3 minor either. Thus, by the induction hypothesis,  G/xy  has a drawing ˜

G  in the plane. Let  f  be the face of ˜

G − vxy  containing the

˜

G

point  vxy, and let  C  be the boundary of  f . Let  X :=  NG( x) r  { y }  and f, C

Y :=  NG( y) r  { x }; then  X ∪ Y ⊆ V ( C), because  vxy ∈ f . Clearly, X, Y

˜

G0 := ˜

G − { vxyv | v ∈ Y  r  X }

˜

G0

may be viewed as a drawing of  G − y, in which the vertex  x  is represented by the point  vxy (Fig. 4.4.2). Our aim is to add  y  to this drawing to obtain a drawing of  G.

Since ˜

G  is 3-connected, ˜

G − vxy  is 2-connected, so  C  is a cycle

(Proposition 4.2.5). Let  x 1 , . . . , xk  be an enumeration along this cycle of x 1 , . . . , xk

the vertices in  X, and let  Pi =  xi . . . xi+1 be the  X-paths on  C  between Pi

them ( i = 1 , . . . , k; with  xk+1 :=  x 1). For each  i, the set  C  r  Pi  is contained in one of the two faces of the cycle  Ci :=  xxiPixi+1 x; we Ci

82

4. Planar Graphs

x 1

x 5

f 1

P 4

x 2

x (=  vxy)

f

C

x 4

x 3

Fig. 4.4.2. ˜

G0  as a drawing of  G − y: the vertex  x  is represented by the point  vxy

fi

denote the  other  face of  Ci  by  fi. Since  fi  contains points of  f (close to  x) but no points of  C, we have  fi ⊆ f . Moreover, the plane edges  xxj with  j /

∈ { i, i + 1  }  meet  Ci  only in  x  and end outside  fi  in  C  r  Pi, so  fi meets none of those edges. Hence  fi ⊆  R2 r ˜

G0, that is,  fi  is contained

in a face of ˜

G0. (In fact,  fi  is a face of ˜

G0, but we do not need this.)

In order to turn ˜

G0  into a drawing of  G, let us try to find an  i

such that  Y ⊆ V ( Pi); we may then embed  y  into  fi  and link it up to its neighbours by arcs inside  fi. Suppose there is no such  i: how then can the vertices of  Y  be distributed around  C? If  y  had a neighbour in some ˚

Pi, it would also have one in  C − Pi, so  G  would contain a T K 3 ,  3 (with branch vertices  x,  y,  xi,  xi+1 and those two neighbours of  y). Hence  Y ⊆ X. Now if  |Y | =  |Y ∩ X| > 3, we have a  T K 5 in  G.

So  |Y |  6 2; in fact,  |Y | = 2, because  d( y) >  κ( G) > 3. Since the two vertices of  Y  lie on no common  Pi, we can once more find a  T K 3 ,  3 in  G, a contradiction.

¤

Compared with other proofs of Kuratowski’s theorem, the above

proof has the attractive feature that it can easily be adapted to produce a drawing in which every inner face is convex (exercise); in particular, every edge can be drawn straight. Note that 3-connectedness is essential here: a 2-connected planar graph need not have a drawing with all inner faces convex (example?), although it always has a straight-line drawing

(Exercise 12).

It is not difficult, in principle, to reduce the general Kuratowski

theorem to the 3-connected case by manipulating and combining partial

drawings assumed to exist by induction. For example, if  κ( G) = 2 and G =  G 1  ∪ G 2 with  V ( G 1  ∩ G 2) =  { x, y }, and if  G  has no  T K 5 or  T K 3 ,  3

subgraph, then neither  G 1 +  xy  nor  G 2 +  xy  has such a subgraph, and we may try to combine drawings of these graphs to one of  G +  xy. (If xy  is already an edge of  G, the same can be done with  G 1 and  G 2.) For  κ( G) 6 1, things become even simpler. However, the geometric operations involved require some cumbersome shifting and scaling, even

if all the plane edges occurring are assumed to be straight.

4.4 Planar graphs: Kuratowski’s theorem

83

The following more combinatorial route is just as easy, and may be

a welcome alternative.

Lemma 4.4.4.  Let X be a set of 3-connected graphs. Let G be a graph

[ 8.3.1 ]

with κ( G) 6 2 , and let G 1 , G 2  be proper induced subgraphs of G such that G =  G 1  ∪ G 2  and |G 1  ∩ G 2 | =  κ( G) . If G is edge-maximal without a topological minor in X , then so are G 1  and G 2 , and G 1  ∩ G 2 =  K 2 .

Proof . Note first that every vertex  v ∈ S :=  V ( G 1  ∩ G 2) has a neigh-S

bour in every component of  Gi − S,  i = 1 ,  2: otherwise  S  r  { v }  would separate  G, contradicting  |S| =  κ( G). By the maximality of  G, every edge  e  added to  G  lies in a  T X ⊆ G +  e  with  X ∈ X . For all the X

choices of  e  considered below, the 3-connectedness of  X  will imply that the branch vertices of this  T X  all lie in the same  Gi, say in  G 1. (The position of  e  will always be symmetrical with respect to  G 1 and  G 2, so this assumption entails no loss of generality.) Then the  T X  meets  G 2 at most in a path  P  corresponding to an edge of  X.

P

If  S =  ∅, we obtain an immediate contradiction by choosing  e  with one end in  G 1 and the other in  G 2. If  S =  { v }  is a singleton, let  e join a neighbour  v 1 of  v  in  G 1  − S  to a neighbour  v 2 of  v  in  G 2  − S

(Fig. 4.4.3). Then  P  contains both  v  and the edge  v 1 v 2; replacing  vP v 1

with the edge  vv 1, we obtain a  T X  in  G 1  ⊆ G, a contradiction.

e

P

v 1

v 2

T X

v

G 1

G 2

Fig. 4.4.3.  If  G +  e  contains a  T X, then so does  G 1 or  G 2

So  |S| = 2, say  S =  { x, y }. If  xy /

∈ G, we let  e :=  xy, and in the

x, y

arising  T X  replace  e  by an  x– y  path through  G 2; this gives a  T X  in  G, a contradiction. Hence  xy ∈ G, and  G [  S ] =  K 2 as claimed.

It remains to show that  G 1 and  G 2 are edge-maximal without a topological minor in  X . So let  e0  be an additional edge for  G 1, say.

Replacing  xP y  with the edge  xy  if necessary, we obtain a  T X  either in  G 1 +  e0 (which shows the edge-maximality of  G 1, as desired) or in  G 2

(which contradicts  G 2  ⊆ G).

¤

Lemma 4.4.5.  If |G| > 4  and G is edge-maximal with T K 5 , T K 3 ,  3  6⊆ G, then G is 3-connected.

84

4. Planar Graphs

Proof . We apply induction on  |G|. For  |G| = 4, we have  G =  K 4

(4.2.9)

and the assertion holds. Now let  |G| >  4, and let  G  be edge-maximal G 1 , G 2

without a  T K 5 or  T K 3 ,  3. Suppose  κ( G) 6 2, and choose  G 1 and  G 2 as in Lemma 4.4.4. For  X :=  { K 5 , K 3 ,  3  }, the lemma says that  G 1  ∩ G 2 is x, y

a  K 2, with vertices  x, y  say. By Lemmas 4.4.4, 4.4.3 and the induction hypothesis,  G 1 and  G 2 are planar. For each  i = 1 ,  2 separately, choose a fi

drawing of  Gi, a face  fi  with the edge  xy  on its boundary, and a vertex zi

zi 6=  x, y  on the boundary of  fi. Let  K  be a  T K 5 or  T K 3 ,  3 in the K

abstract graph  G +  z 1 z 2 (Fig. 4.4.4).

z 1

x

z 2

K

y

G 1

G 2

Fig. 4.4.4.  A  T K 5 or  T K 3 ,  3 in  G +  z 1 z 2

If all the branch vertices of  K  lie in the same  Gi, then either  Gi +  xzi or  Gi +  yzi (or  Gi  itself, if  zi  is already adjacent to  x  or  y, respectively) contains a  T K 5 or  T K 3 ,  3; this contradicts Corollary 4.2.9, since these graphs are planar by the choice of  zi. Since  G +  z 1 z 2 does not contain four independent paths between ( G 1  − G 2) and ( G 2  − G 1), these subgraphs cannot both contain a branch vertex of a  T K 5, and cannot both contain two branch vertices of a  T K 3 ,  3. Hence  K  is a  T K 3 ,  3 with only one branch vertex  v  in, say,  G 2  − G 1. But then also the graph  G 1 +  v +  { vx, vy, vz 1  }, which is planar by the choice of  z 1, contains a  T K 3 ,  3. This contradicts

Corollary 4.2.9.

¤

Theorem 4.4.6. (Kuratowski 1930; Wagner 1937)

[ 4.5.1 ]

The following assertions are equivalent for graphs G:

[ 12.4.3 ]

(i)  G is planar;

(ii)  G contains neither K 5  nor K 3 ,  3  as a minor; (iii)  G contains neither K 5  nor K 3 ,  3  as a topological minor.

(4.2.9)

Proof . Combine Corollary 4.2.9 and Proposition 4.4.2 with Lemmas

4.4.3 and 4.4.5.

¤

Corollary 4.4.7.  Every maximal planar graph with at least four vertices is 3-connected.

Proof . Apply Lemma 4.4.5 and Theorem 4.4.6.

¤

4.5. Algebraic planarity criteria

85

4.5 Algebraic planarity criteria

In this section we show that planarity can be characterized in purely

algebraic terms, by a certain abstract property of its cycle space. Theorems relating such seemingly distant graph properties are rare, and their significance extends beyond their immediate applicability. In a sense,

they indicate that both ways of viewing a graph—in our case, the topo-

logical and the algebraic way—are not just formal curiosities: if both are natural enough that, quite unexpectedly, each can be expressed in terms of the other, the indications are that they have the power to reveal some genuine insights into the structure of graphs and are worth pursuing.

Let  G = ( V, E) be a graph. We call a subset  F  of its edge space E( G)  simple  if every edge of  G  lies in at most two sets of  F. For example, simple

the cut space  C∗( G) has a simple basis: according to Proposition 1.9.3 it is generated by the cuts  E( v) formed by all the edges at a given vertex  v, and an edge  xy ∈ G  lies in  E( v) only for  v =  x  and for  v =  y.

Theorem 4.5.1. (MacLane 1937)

A graph is planar if and only if its cycle space has a simple basis.

[ 4.6.3 ]

Proof . The assertion being trivial for graphs of order at most 2, we

(1.9.2)

consider a graph  G  of order at least 3. If  κ( G) 6 1, then  G  is the union

(1.9.6)

of two proper induced subgraphs  G 1 , G 2 with  |G 1  ∩ G 2 |  6 1. Then  C( G)

(4.1.1)

(4.2.1)

is the direct sum of  C( G 1) and  C( G 2), and hence has a simple basis if

(4.2.5)

and only if both  C( G 1) and  C( G 2) do (proof?). Moreover,  G  is planar if

(4.4.6)

and only if both  G 1 and  G 2 are: this follows at once from Kuratowski’s

theorem, but also from easy geometrical considerations. The assertion for  G  thus follows inductively from those for  G 1 and  G 2. For the rest of the proof, we now assume that  G  is 2-connected.

We first assume that  G  is planar and choose a drawing. By Lemma

4.2.5, the face boundaries of  G  are cycles, so they are elements of  C( G).

We shall show that the face boundaries generate all the cycles in  G; then C( G) has a simple basis by Lemma 4.2.1. Let  C ⊆ G  be any cycle, and let  f  be its inner face. By Lemma 4.2.1, every edge  e  with ˚

e ⊆ f  lies on

exactly two face boundaries  G [  f 0 ] with  f 0 ⊆ f , and every edge of  C  lies on exactly one such face boundary. Hence the sum in  C( G) of all those face boundaries is exactly  C.

Conversely, let  { C 1 , . . . , Ck }  be a simple basis of  C( G). Then, for every edge  e ∈ G, also  C( G − e) has a simple basis. Indeed, if  e  lies in just one of the sets  Ci, say in  C 1, then  { C 2 , . . . , Ck }  is a simple basis of  C( G − e); if  e  lies in two of the  Ci, say in  C 1 and  C 2, then

{ C 1 +  C 2 , C 3 , . . . , Ck }  is such a basis. (Note that the two bases are indeed subsets of  C( G − e) by Proposition 1.9.2.) Thus every subgraph of  G  has a cycle space with a simple basis. For our proof that  G  is planar, it thus suffices to show that the cycle spaces of  K 5 and  K 3 ,  3 (and hence

86

4. Planar Graphs

those of their subdivisions) do  not  have a simple basis: then  G  cannot contain a  T K 5 or  T K 3 ,  3, and so is planar by Kuratowski’s theorem.

Let us consider  K 5 first. By Theorem 1.9.6, dim  C( K 5) = 6; let B =  { C 1 , . . . , C 6  }  be a simple basis, and put  C 0 :=  C 1 +  . . . +  C 6. As B  is linearly independent, none of the sets  C 0 , . . . , C 6 is empty, and so each of them contains at least three edges (cf. Proposition 1.9.2). The simplicity of  B  therefore implies

18 = 6  ·  3 6  |C 1 | +  . . . +  |C 6 |

6 2  kK 5 k − |C 0 |

6 2  ·  10  −  3 = 17  ,

a contradiction; for the middle inequality note that every edge in  C 0 lies in just one of the sets  C 1 , . . . , C 6.

For  K 3 ,  3, Theorem 1.9.6 gives dim  C( K 3 ,  3) = 4; let  B =  { C 1 , . . . , C 4  }

be a simple basis, and put  C 0 :=  C 1 +  . . . +  C 4. Since  K 3 ,  3 has girth 4, each  Ci  contains at least four edges, so

16 = 4  ·  4 6  |C 1 | +  . . . +  |C 4 |

6 2  kK 3 ,  3 k − |C 0 |

6 2  ·  9  −  4 = 14  ,

a contradiction.

¤

It is one of the hidden beauties of planarity theory that two such

abstract and seemingly unintuitive results about generating sets in cy-

cle spaces as MacLane’s theorem and Tutte’s theorem 3.2.3 conspire to produce a very tangible planarity criterion for 3-connected graphs:

Theorem 4.5.2. (Tutte 1963)

A 3-connected graph is planar if and only if every edge lies on at most (equivalently: exactly) two non-separating induced cycles.

(3.2.3)

(4.2.1)

Proof . The forward implication follows from Propositions 4.2.10 and

(4.2.5)

(4.2.10)

4.2.1 (and Proposition 4.2.5 for the ‘exactly two’ version); the backward implication follows from Theorems 3.2.3 and 4.5.1.

¤

4.6 Plane duality

87

4.6 Plane duality

In this section we shall use MacLane’s theorem to uncover another connection between planarity and algebraic structure: a connection between the duality of plane graphs, defined below, and the duality of the cycle and cut space hinted at in Chapters 1.9 and 3.5.

plane

A  plane multigraph  is a pair  G = ( V, E) of finite sets (of  vertices multigraph

and  edges, respectively) satisfying the following conditions:

(i)  V ⊆  R2;

(ii) every edge is either an arc between two vertices or a polygon

containing exactly one vertex (its  endpoint );

(iii) apart from its own endpoint(s), an edge contains no vertex and

no point of any other edge.

We shall use terms defined for plane graphs freely for plane multigraphs.

Note that, as in abstract multigraphs, both loops and double edges count as cycles.

Let us consider the plane multigraph  G  shown in Figure 4.6.1. Let us place a new vertex inside each face of  G  and link these new vertices up to form another plane multigraph  G∗, as follows: for every edge  e  of G  we link the two new vertices in the faces incident with  e  by an edge  e∗

crossing  e; if  e  is incident with only one face, we attach a loop  e∗  to the new vertex in that face, again crossing the edge  e. The plane multigraph G∗  formed in this way is then dual to  G  in the following sense: if we apply the same procedure as above to  G∗, we obtain a plane multigraph very similar to  G; in fact,  G  itself may be reobtained from  G∗  in this way.

e∗

e

G

G∗

Fig. 4.6.1.  A plane graph and its dual

To make this idea more precise, let  G = ( V, E) and ( V ∗, E∗) be any two plane multigraphs, and put  F ( G) =:  F  and  F (( V ∗, E∗)) =:  F ∗. We plane

call ( V ∗, E∗) a  plane dual  of  G, and write ( V ∗, E∗) =:  G∗, if there are dual G∗

bijections

F → V ∗

E → E∗

V → F ∗

f 7→ v∗( f )

e 7→ e∗

v 7→ f ∗( v)

satisfying the following conditions:

(i)  v∗( f )  ∈ f  for all  f ∈ F ;

88

4. Planar Graphs

(ii)  |e∗ ∩ G| =  |˚

e∗ ∩˚

e| =  |e ∩ G∗| = 1 for all  e ∈ E;

(iii)  v ∈ f ∗( v) for all  v ∈ V .

The existence of such bijections implies that both  G  and  G∗  are connected (exercise). Conversely, every connected plane multigraph  G  has a plane dual  G∗: if we pick from each face  f  of  G  a point  v∗( f ) as a vertex for  G∗, we can always link these vertices up by independent arcs as required by condition (ii), and there is always a bijection  V → F ∗

satisfying (iii) (exercise).

If  G∗

'

1 and  G∗

2 are two plane duals of  G, then clearly  G∗

1

G∗ 2; in fact,

one can show that the natural bijection  v∗ 1( f)  7→ v∗ 2( f) is a topological isomorphism between  G∗ 1 and  G∗ 2. In this sense, we may speak of  the plane dual  G∗  of  G.

Finally,  G  is in turn a plane dual of  G∗. Indeed, this is witnessed by the inverse maps of the bijections from the definition of  G∗: setting v∗( f ∗( v)) :=  v  and  f ∗( v∗( f )) :=  f  for  f ∗( v)  ∈ F ∗  and  v∗( f )  ∈ V ∗, we see that conditions (i) and (iii) for  G∗  transform into (iii) and (i) for  G, while condition (ii) is symmetrical in  G  and  G∗. Thus, the term ‘dual’

is also formally justified.

Plane duality is fascinating not least because it establishes a con-

nection between two natural but very different kinds of edge sets in a

multigraph, between cycles and cuts:

[ 6.5.2 ]

Proposition 4.6.1.  For any connected plane multigraph G, an edge set E ⊆ E( G)  is the edge set of a cycle in G if and only if E∗ :=  { e∗ | e ∈ E }

is a minimal cut in G∗.

(4.1.1)

Proof . By conditions (i) and (ii) in the definition of  G∗, two vertices

(4.2.3)

v∗( f 1) and  v∗( f 2) of  G∗  lie in the same component of  G∗ − E∗  if and S

only if  f 1 and  f 2 lie in the same region of R2 r

E: every  v∗( f 1)– v∗( f 2)

S

path in  G∗− E∗  is an arc between  f 1 and  f 2 in R2 r E, and conversely

every such arc  P (with  P ∩ V ( G) =  ∅) defines a walk in  G∗− E∗  between v∗( f 1) and  v∗( f 2).

Now if  C ⊆ G  is a cycle and  E =  E( C) then, by the Jordan curve

theorem and the above correspondence,  G∗− E∗  has exactly two components, so  E∗  is a minimal cut in  G∗.

Conversely, if  E ⊆ E( G) is such that  E∗  is a cut in  G∗, then, by

Proposition 4.2.3 and the above correspondence,  E  contains the edges of a cycle  C ⊆ G. If  E∗  is minimal as a cut, then  E  cannot contain any further edges (by the implication shown before), so  E =  E( C).

¤

Proposition 4.6.1 suggests the following generalization of plane du-

ality to a notion of duality for abstract multigraphs. Let us call a multi-abstract

graph  G∗  an  abstract dual  of a multigraph  G  if  E( G∗) =  E( G) and the dual

minimal cuts in  G∗  are precisely the edge sets of cycles in  G. Note that any abstract dual of a multigraph is connected.

4.6 Plane duality

89

Proposition 4.6.2.  If G∗ is an abstract dual of G, then the cut space of G∗ is the cycle space of G, i.e.

C∗( G∗) =  C( G)  .

Proof . By Lemma 1.9.4,5  C∗( G∗) is the subspace of  E( G∗) =  E( G)

(1.9.4)

generated by the minimal cuts in  G∗. By assumption, these are precisely the edge sets of the cycles in  G, and these generate  C( G) in  E( G).

¤

We finally come to one of the highlights of classical planarity the-

ory: the planar graphs are characterized by the fact that they have an

abstract dual. Although less obviously intuitive, this duality is just as fundamental a property as planarity itself; indeed the following theorem may well be seen as a topological characterization of the graphs that

have a dual:

Theorem 4.6.3. (Whitney 1933)

A graph is planar if and only if it has an abstract dual.

Proof . Let  G  be a graph. If  G  is plane, then every component  C  of  G  has

(1.9.3)

(4.5.1)

a plane dual  C∗. Let us consider these  C∗  as abstract multigraphs, pick a vertex in each of them, and identify these vertices. In the connected multigraph  G∗  obtained, the set of minimal cuts is the union of the sets of minimal cuts in the multigraphs  C∗. By Proposition 4.6.1, these cuts are precisely the edge sets of the cycles in  G, so  G∗  is an abstract dual of  G.

Conversely, suppose that  G  has an abstract dual  G∗. By Theorem

4.5.1 and Proposition 4.6.2 it suffices to show that  C∗( G∗) has a simple basis, which it has by Proposition 1.9.3.

¤

Exercises

1.

Show that every graph can be embedded in R3 with all edges straight.

2.  −  Show directly by Lemma 4.1.2 that  K 3 ,  3 is not planar.

3.  −  Find an Euler formula for disconnected graphs.

4.

Show that every connected planar graph with  n  vertices,  m  edges and finite girth  g  satisfies  m  6  g ( n −  2).

g− 2

5.

Show that every planar graph is a union of three forests.

5 Although the lemma was stated for graphs only, its proof remains the same for multigraphs.

90

4. Planar Graphs

6.

Let  G 1 , G 2 , . . .  be an infinite sequence of pairwise non-isomorphic graphs. Show that if lim sup  ε( Gi)  >  3 then the graphs  Gi  have unbounded genus—that is to say, there is no (closed) surface  S  in which all the  Gi  can be embedded.

(Hint. You may use the fact that for every surface  S  there is a constant χ( S) 6 2 such that every graph embedded in  S  satisfies the generalized Euler formula of  n − m +  ` >  χ( S).)

7.

Find a direct proof for planar graphs of Tutte’s theorem on the cycle

space of 3-connected graphs (Theorem 3.2.3).

8.  −  Show that the two plane graphs in Fig. 4.3.1 are not combinatorially (and hence not topologically) isomorphic.

9.

Show that the two graphs in Fig. 4.3.2 are combinatorially but not topologically isomorphic.

10.  −  Show that our definition of equivalence for planar embeddings does indeed define an equivalence relation.

11.

Find a 2-connected planar graph whose drawings are all topologically

isomorphic but whose planar embeddings are not all equivalent.

12.+ Show that every plane graph is combinatorially isomorphic to a plane graph whose edges are all straight.

(Hint.

Given a plane triangulation, construct inductively a graph-

theoretically isomorphic plane graph whose edges are straight. Which

additional property of the inner faces could help with the induction?)

Do not use Kuratowski’s theorem in the following two exercises.

13.

Show that any minor of a planar graph is planar. Deduce that a graph

is planar if and only if it is the minor of a grid. ( Grids  are defined in Chapter 12.3.)

14.

(i) Show that the planar graphs can in principle be characterized as

in Kuratowski’s theorem, i.e., that there exists a set  X  of graphs such that a graph  G  is planar if and only if  G  has no topological minor in  X .

(ii) More generally, which graph properties can be characterized in this way?

15.  −  Does every planar graph have a drawing with all inner faces convex?

16.

Modify the proof of Lemma 4.4.3 so that all inner faces become convex.

17.

Does every minimal non-planar graph  G (i.e., every non-planar graph  G

whose proper subgraphs are all planar) contain an edge  e  such that G − e  is maximally planar? Does the answer change if we define ‘minimal’ with respect to minors rather than subgraphs?

18.

Show that adding a new edge to a maximal planar graph of order at

least 6 always produces both a  T K 5 and a  T K 3 ,  3 subgraph.

Exercises

91

19.

Prove the general Kuratowski theorem from its 3-connected case by manipulating plane graphs, i.e. avoiding Lemma 4.4.5.

(This is not intended as an exercise in elementary topology; for the

topological parts of the proof, a rough sketch will do.)

20.

A graph is called  outerplanar  if it has a drawing in which every vertex lies on the boundary of the outer face. Show that a graph is outerplanar if and only if it contains neither  K 4 nor  K 2 ,  3 as a minor.

21.

Let  G =  G 1  ∪ G 2, where  |G 1  ∩ G 2 |  6 1. Show that  C( G) has a simple basis if both  C( G 1) and  C( G 2) have one.

22.+ Find a cycle space basis among the face boundaries of a 2-connected plane graph.

23.

Show that a 2-connected plane graph is bipartite if and only if every

face is bounded by an even cycle.

24.  −  Let  G  be a connected plane multigraph, and let  G∗  be its plane dual.

Prove the following two statements for every edge  e ∈ G:

(i) If  e  lies on the boundary of two distinct faces  f 1 , f 2 of  G, then e∗ =  v∗( f 1)  v∗( f 2).

(ii) If  e  lies on the boundary of exactly one face  f  of  G, then  e∗  is a loop at  v∗( f ).

25.  −  What does the plane dual of a plane tree look like?

26.  −  Show that the plane dual of a plane multigraph is connected.

27.+ Show that a plane multigraph has a plane dual if and only if it is connected.

28.

Let  G, G∗  be mutually dual plane multigraphs, and let  e ∈ E( G). Prove the following statements (with a suitable definition of  G/e):

(i) If  e  is not a bridge, then  G∗/e∗  is a plane dual of  G − e.

(ii) If  e  is not a loop, then  G∗ − e∗  is a plane dual of  G/e.

29.

Show that any two plane duals of a plane multigraph are combinatori-

ally isomorphic.

30.

Let  G, G∗  be mutually dual plane graphs. Prove the following statements:

(i) If  G  is 2-connected, then  G∗  is 2-connected.

(ii) If  G  is 3-connected, then  G∗  is 3-connected.

(iii) If  G  is 4-connected, then  G∗  need not be 4-connected.

31.

Let  G, G∗  be mutually dual plane graphs. Let  B 1 , . . . , Bn  be the blocks of  G. Show that  B∗

1  , . . . , B∗

n  are the blocks of  G∗.

32.

Show that if  G∗  is an abstract dual of a multigraph  G, then  G  is an abstract dual of  G∗.

92

4. Planar Graphs

33.

Show that a connected graph  G = ( V, E) is planar if and only if there exists a connected multigraph  G0 = ( V 0, E) (i.e. with the same edge set) such that the following holds for every set  F ⊆ E: the graph ( V, F ) is a tree if and only if ( V 0, E  r  F ) is a tree.

Notes

There is an excellent monograph on the embedding of graphs in surfaces, including the plane: B. Mohar & C. Thomassen,  Graphs on Surfaces, Johns Hopkins University Press, to appear. Proofs of the results cited in Section 4.1, as well as all references for this chapter, can be found there. A good account of the Jordan curve theorem, both polygonal and general, is given also in J. Stillwell,  Classical topology and combinatorial group theory, Springer 1980.

The short proof of Corollary 4.2.8 uses a trick that deserves special mention: the so-called  double counting  of pairs, illustrated in the text by a bipartite graph whose edges can be counted alternatively by summing its degrees on the left or on the right. Double counting is a technique widely used in combinatorics, and there will be more examples later in the book.

The material of Section 4.3 is not normally standard for an introductory graph theory course, and the rest of the chapter can be read independently of this section. However, the results of Section 4.3 are by no means unimportant.

In a way, they have fallen victim to their own success: the shift from a topological to a combinatorial setting for planarity problems which they achieve has made the topological techniques developed there dispensable for most of planarity theory.

In its original version, Kuratowski’s theorem was stated only for topological minors; the version for general minors was added by Wagner in 1937.

Our proof of the 3-connected case (Lemma 4.4.3) can easily be strengthened to make all the inner faces convex (exercise); see C. Thomassen, Planarity and duality of finite and infinite graphs,  J. Combin. Theory B 29 (1980), 244–271.

The existence of such ‘convex’ drawings for 3-connected planar graphs follows already from the theorem of Steinitz (1922) that these graphs are precisely the 1-skeletons of 3-dimensional convex polyhedra. Compare also W.T. Tutte, How to draw a graph,  Proc. London Math. Soc. 13 (1963), 743–767.

As one readily observes, adding an edge to a maximal planar graph (of

order at least 6) produces not only a topological  K 5  or K 3 ,  3, but both. In Chapter 8.3 we shall see that, more generally, every graph with  n  vertices and more than 3 n −  6 edges contains a  T K 5 and, with one easily described class of exceptions, also a  T K 3 ,  3. Seymour conjectures that every 5-connected non-planar graph contains a  T K 5 (unpublished).

The simple cycle space basis constructed in the proof of MacLane’s theorem, which consists of the inner face boundaries, is canonical in the following sense: for every simple basis  B  of the cycle space of a 2-connected planar graph there exists a drawing of that graph in which  B  is precisely the set of inner face boundaries. (This is proved in Mohar & Thomassen, who also mention some further planarity criteria.) Our proof of the backward direction of MacLane’s

theorem is based on Kuratowski’s theorem. A more direct approach, in which

Notes

93

a planar embedding is actually constructed from a simple basis, is adopted in K. Wagner,  Graphentheorie, BI Hochschultaschenbücher 1972.

The proper setting for duality phenomena between cuts and cycles in ab-

stract graphs (and beyond) is the theory of  matroids; see J.G. Oxley,  Matroid Theory, Oxford University Press 1992.

5

Colouring

How many colours do we need to colour the countries of a map in such

a way that adjacent countries are coloured differently? How many days

have to be scheduled for committee meetings of a parliament if every

committee intends to meet for one day and some members of parliament

serve on several committees? How can we find a school timetable of min-

imum total length, based on the information of how often each teacher

has to teach each class?

vertex

A  vertex colouring  of a graph  G = ( V, E) is a map  c:  V → S  such colouring

that  c( v)  6=  c( w) whenever  v  and  w  are adjacent. The elements of the set  S  are called the available  colours. All that interests us about  S  is its size: typically, we shall be asking for the smallest integer  k  such that chromatic

G  has a  k-colouring, a vertex colouring  c:  V → {  1 , . . . , k }. This  k  is the number

( vertex-)  chromatic number  of  G; it is denoted by  χ( G). A graph  G  with χ( G)

χ( G) =  k  is called  k-chromatic; if  χ( G) 6  k, we call  G k- colourable.

2

3

1

4

1

2

Fig. 5.0.1.  A vertex colouring  V → {  1 , . . . ,  4  }

Note that a  k-colouring is nothing but a vertex partition into  k colour

independent sets, now called  colour classes; the non-trivial 2-colourable classes

graphs, for example, are precisely the bipartite graphs. Historically,

the colouring terminology comes from the map colouring problem stated

96

5. Colouring

above, which leads to the problem of determining the maximum chro-

matic number of planar graphs. The committee scheduling problem, too,

can be phrased as a vertex colouring problem—how?

edge

An  edge colouring  of  G = ( V, E) is a map  c:  E → S  with  c( e)  6=  c( f ) colouring

for any adjacent edges  e, f . The smallest integer  k  for which a  k-edge-colouring  exists, i.e. an edge colouring  c:  E

chromatic

→ {  1 , . . . , k }, is the  edge-

index

chromatic number , or  chromatic index , of  G; it is denoted by  χ0( G).

χ0( G)

The third of our introductory questions can be modelled as an edge

colouring problem in a bipartite multigraph (how?).

Clearly, every edge colouring of  G  is a vertex colouring of its line graph  L( G), and vice versa; in particular,  χ0( G) =  χ( L( G)). The problem of finding good edge colourings may thus be viewed as a restriction of the more general vertex colouring problem to this special class of

graphs. As we shall see, this relationship between the two types of

colouring problem is reflected by a marked difference in our knowledge

about their solutions: while there are only very rough estimates for  χ, its sister  χ0  always takes one of two values, either ∆ or ∆ + 1.

5.1 Colouring maps and planar graphs

If any result in graph theory has a claim to be known to the world

outside, it is the following  four colour theorem (which implies that every map can be coloured with at most four colours):

Theorem 5.1.1. (Four Colour Theorem)

Every planar graph is 4-colourable.

Some remarks about the proof of the four colour theorem and its history can be found in the notes at the end of this chapter. Here, we prove the following weakening:

Proposition 5.1.2. (Five Colour Theorem)

Every planar graph is 5-colourable.

(4.1.1)

Proof . Let  G  be a plane graph with  n > 6 vertices and  m  edges. We

(4.2.8)

assume inductively that every plane graph with fewer than  n  vertices n, m

can be 5-coloured. By Corollary 4.2.8,

d( G) = 2 m/n  6 2 (3 n −  6) /n <  6 ; v

let  v ∈ G  be a vertex of degree at most 5. By the induction hypothesis, H

the graph  H :=  G − v  has a vertex colouring  c:  V ( H)  → {  1 , . . . ,  5  }. If  c c

uses at most 4 colours for the neighbours of  v, we can extend it to a 5-colouring of  G. Let us assume, therefore, that  v  has exactly 5 neighbours, and that these have distinct colours.

5.1 Colouring maps and planar graphs

97

Let  D  be an open disc around  v, so small that it meets only those D

five straight edge segments of  G  that contain  v. Let us enumerate these segments according to their cyclic position in  D  as  s 1 , . . . , s 5, and let s 1 , . . . , s 5

vvi  be the edge containing  si ( i = 1 , . . . ,  5; Fig. 5.1.1). Without loss of v 1 , . . . , v 5

generality we may assume that  c( vi) =  i  for each  i.

v 1

s 1

v 2

s 2

P

v

s 5

5

v

D

s 3

s 4

v 3

v 4

Fig. 5.1.1.  The proof of the five colour theorem

Let us show first that every  v 1–  v 3 path  P ⊆ H  separates  v 2 from P

v 4 in  H. Clearly, this is the case if and only if the cycle  C :=  vv 1 P v 3 v C

separates  v 2 from  v 4 in  G. We prove this by showing that  v 2 and  v 4 lie in different faces of  C.

Consider the two regions of  D  r ( s 1  ∪ s 3). One of these regions meets  s 2, the other  s 4. Since  C ∩ D ⊆ s 1  ∪ s 3, the two regions are each contained within a face of  C. Moreover, these faces are distinct: otherwise,  D  would meet only one face of  C, contrary to the fact that v  lies on the boundary of both faces (Theorem 4.1.1). Thus  D ∩ s 2 and D ∩ s 4 lie in distinct faces of  C. As  C  meets the edges  vv 2  ⊇ s 2 and vv 4  ⊇ s 4 only in  v, the same holds for  v 2 and  v 4.

Given  i, j ∈ {  1 , . . . ,  5  }, let  Hi,j  be the subgraph of  H  induced by Hi,j

the vertices coloured  i  or  j. We may assume that the component  C 1 of H 1 ,  3 containing  v 1 also contains  v 3. Indeed, if we interchange the colours 1 and 3 at all the vertices of  C 1, we obtain another 5-colouring of  H; if  v 3  /

∈ C 1, then  v 1 and  v 3 are both coloured 3 in this new colouring, and we may assign colour 1 to  v. Thus,  H 1 ,  3 contains a  v 1–  v 3 path  P .

As shown above,  P  separates  v 2 from  v 4 in  H. Since  P ∩ H 2 ,  4 =  ∅, this means that  v 2 and  v 4 lie in different components of  H 2 ,  4. In the component containing  v 2, we now interchange the colours 2 and 4, thus recolouring  v 2 with colour 4. Now  v  no longer has a neighbour coloured 2, and we may give it this colour.

¤

As a backdrop to the two famous theorems above, let us cite another

well-known result:

Theorem 5.1.3. (Grötzsch 1959)

Every planar graph not containing a triangle is 3-colourable.

98

5. Colouring

5.2 Colouring vertices

How do we determine the chromatic number of a given graph? How can

we  find  a vertex-colouring with as few colours as possible? How does the chromatic number relate to other graph invariants, such as average

degree, connectivity or girth?

Straight from the definition of the chromatic number we may derive

the following upper bound:

Proposition 5.2.1.  Every graph G with m edges satisfies

q

χ( G) 6 1 +

2 m + 1  .

2

4

Proof . Let  c  be a vertex colouring of  G  with  k =  χ( G) colours. Then G  has at least one edge between any two colour classes: if not, we could have used the same colour for both classes. Thus,  m > 1  k( k −  1). Solving 2

this inequality for  k, we obtain the assertion claimed.

¤

One obvious way to colour a graph  G  with not too many colours is greedy

the following  greedy algorithm: starting from a fixed vertex enumeration algorithm

v 1 , . . . , vn  of  G, we consider the vertices in turn and colour each  vi  with the first available colour—e.g., with the smallest positive integer not already used to colour any neighbour of  vi  among  v 1 , . . . , vi− 1. In this way, we never use more than ∆( G) + 1 colours, even for unfavourable choices of the enumeration  v 1 , . . . , vn. If  G  is complete or an odd cycle, then this is even best possible.

In general, though, this upper bound of ∆ + 1 is rather generous,

even for greedy colourings. Indeed, when we come to colour the vertex

vi  in the above algorithm, we only need a supply of  dG[  v 1 ,...,vi ]( vi) + 1

rather than  dG( vi) + 1 colours to proceed; recall that, at this stage, the algorithm ignores any neighbours  vj  of  vi  with  j > i. Hence in most graphs, there will be scope for an improvement of the ∆ + 1 bound by choosing a particularly suitable vertex ordering to start with: one that picks vertices of large degree early (when most neighbours are ignored) and vertices

of small degree last. Locally, the number  dG[  v 1 ,...,vi ]( vi) + 1 of colours required will be smallest if  vi  has minimum degree in  G [  v 1 , . . . , vi ]. But this is easily achieved: we just choose  vn  first, with  d( vn) =  δ( G), then choose as  vn− 1 a vertex of minimum degree in  G − vn, and so on.

The least number  k  such that  G  has a vertex enumeration in which each vertex is preceded by fewer than  k  of its neighbours is called colouring

number

the  colouring number  col( G) of  G. The enumeration we just discussed col( G)

shows that col( G) 6 max H⊆G δ( H) + 1. But for  H ⊆ G  clearly also col( G) > col( H) and col( H) >  δ( H) + 1, since the ‘back-degree’ of the last vertex in any enumeration of  H  is just its ordinary degree in  H, which is at least  δ( H). So we have proved the following:

5.2 Colouring vertices

99

Proposition 5.2.2.  Every graph G satisfies

χ( G) 6 col( G) = max  { δ( H)  | H ⊆ G } + 1  .

¤

[ 9.2.1 ]

Corollary 5.2.3.  Every graph G has a subgraph of minimum degree at

[ 9.2.3 ]

[ 11.2.3 ]

least χ( G)  −  1 .

¤

The colouring number of a graph is closely related to its arboricity; see

the remark following Theorem 3.5.4.

As we have seen, every graph  G  satisfies  χ( G) 6 ∆( G) + 1, with equality for complete graphs and odd cycles. In all other cases, this

general bound can be improved a little:

Theorem 5.2.4. (Brooks 1941)

Let G be a connected graph. If G is neither complete nor an odd cycle, then

χ( G) 6 ∆( G)  .

Proof . We apply induction on  |G|. If ∆( G) 6 2, then  G  is a path or a cycle, and the assertion is trivial. We therefore assume that ∆ :=

∆

∆( G) > 3, and that the assertion holds for graphs of smaller order.

Suppose that  χ( G)  > ∆.

Let  v ∈ G  be a vertex and  H :=  G − v. Then  χ( H) 6 ∆ : by v, H

induction, every component  H0  of  H  satisfies  χ( H0) 6 ∆( H0) 6 ∆ unless H0  is complete or an odd cycle, in which case  χ( H0) = ∆( H0) + 1 6 ∆

as every vertex of  H0  has maximum degree in  H0  and one such vertex is also adjacent to  v  in  G.

Since  H  can be ∆-coloured but  G  cannot, we have the following: Every ∆ -colouring of H uses all the colours  1 , . . . , ∆  on (1)

the neighbours of v; in particular, d( v) = ∆ .

Given any ∆-colouring of  H, let us denote the neighbour of  v  coloured  i  by  vi,  i = 1 , . . . , ∆. For all  i 6=  j, let  Hi,j  denote the subgraph v 1 , . . . , v∆

of  H  spanned by all the vertices coloured  i  or  j.

Hi,j

For all i 6=  j, the vertices vi and vj lie in a common com-

(2)

ponent Ci,j of Hi,j.

Ci,j

Otherwise we could interchange the colours  i  and  j  in one of those components; then  vi  and  vj  would be coloured the same, contrary to (1).

Ci,j is always a vi– vj path.

(3)

Indeed, let  P  be a  vi–  vj  path in  Ci,j. As  dH ( vi) 6 ∆  −  1, the neighbours of  vi  have pairwise different colours: otherwise we could recolour  vi,

100

5. Colouring

contrary to (1). Hence the neighbour of  vi  on  P  is its only neighbour in  Ci,j, and similarly for  vj. Thus if  Ci,j 6=  P , then  P  has an inner vertex with three identically coloured neighbours in  H; let  u  be the first such vertex on  P (Fig. 5.2.1). Since at most ∆  −  2 colours are used on the neighbours of  u, we may recolour  u. But this makes  P˚

u  into a

component of  Hi,j, contradicting (2).

vj

i

Ci,j

j

j

i

v

j

i

u

i

i

j

vi

P˚

u

Fig. 5.2.1.  The proof of (3) in Brooks’s theorem

For distinct i, j, k, the paths Ci,j and Ci,k meet only in vi.

(4)

For if  vi 6=  u ∈ Ci,j ∩ Ci,k, then  u  has two neighbours coloured  j  and two coloured  k, so we may recolour  u. In the new colouring,  vi  and  vj  lie in different components of  Hi,j, contrary to (2).

The proof of the theorem now follows easily. If the neighbours of  v are pairwise adjacent, then each has ∆ neighbours in  N ( v)  ∪{ v }  already, so  G =  G [  N ( v)  ∪ { v } ] =  K∆+1. As  G  is complete, there is nothing to v 1 , . . . , v∆

show. We may thus assume that  v 1 v 2  /

∈ G, where  v 1 , . . . , v∆ derive their

c

names from some fixed ∆-colouring  c  of  H. Let  u 6=  v 2 be the neighbour u

of  v 1 on the path  C 1 ,  2; then  c( u) = 2. Interchanging the colours 1 and 3

c0

in  C 1 ,  3, we obtain a new colouring  c0  of  H; let  v0,  H0 ,  C0  etc. be defined i

i,j

i,j

with respect to  c0  in the obvious way. As a neighbour of  v 1 =  v0 3, our vertex  u  now lies in  C0 2 ,  3 , since  c0( u) =  c( u) = 2. By (4) for  c, however, the path ˚

v 1 C 1 ,  2 retained its original colouring, so  u ∈ ˚

v 1 C 1 ,  2  ⊆ C0 1 ,  2.

Hence  u ∈ C0

∩

2 ,  3

C0 1 ,  2, contradicting (4) for  c0.

¤

As we have seen, a graph  G  of large chromatic number must have large maximum degree: at least  χ( G)  −  1. What else can we say about the structure of graphs with large chromatic number?

One obvious possible cause for  χ( G) >  k  is the presence of a  Kk subgraph. This is a local property of  G, compatible with arbitrary values of global invariants such as  ε  and  κ. Hence, the assumption of  χ( G) >  k does not tell us anything about those invariants for  G  itself. It does, however, imply the existence of a subgraph where those invariants are

large: by Corollary 5.2.3,  G  has a subgraph  H  with  δ( H) >  k −  1, and hence by Theorem 1.4.2 a subgraph  H0  with  κ( H0) >  b  1 ( k −  1) c.

4

5.2 Colouring vertices

101

So are those somewhat denser subgraphs the ‘cause’ for the large

value of  χ? Do they, in turn, necessarily contain a graph of high chromatic number—maybe even one from some small collection of  canonical such graphs, such as  Kk? Interestingly, this is not so: those subgraphs of large but ‘constant’ average degree—bounded below only by a function

of  k, not of  |G|—are not nearly dense enough to contain (necessarily) any particular graph of high chromatic number, let alone  Kk.1

Yet even if the above local structures do not appear to help, it

might still be the case that, somehow, a high chromatic number forces

the existence of certain canonical highly chromatic subgraphs. That this is in fact not the case will be our main result in Chapter 11: according to a classic result of Erd˝

os, proved by probabilistic methods,  there are

graphs of arbitrarily large chromatic number and yet arbitrarily large girth (Theorem 11.2.2). Thus given any graph  H  that is not a forest, for every  k ∈  N there are graphs  G  with  χ( G) >  k  but  H 6⊆ G.2

Thus, contrary to our initial guess that a large chromatic number

might always be caused by some dense local substructure, it can in fact occur as a purely global phenomenon: after all, locally (around each

vertex) a graph of large girth looks just like a tree, and is in particular 2-colourable there!

So far, we asked what a high chromatic number implies: it forces

the invariants  δ,  d, ∆ and  κ  up in some subgraph, but it does not imply the existence of any concrete subgraph of large chromatic number. Let

us now consider the converse question: from what assumptions could we

deduce that the chromatic number of a given graph is large?

Short of a concrete subgraph known to be highly chromatic (such

as  Kk), there is little or nothing in sight: no values of the invariants studied so far imply that the graph considered has a large chromatic

number. (Recall the example of  Kn,n.) So what exactly can cause high chromaticity as a global phenomenon largely remains a mystery!

Nevertheless, there exists a simple—though not always short—

procedure to construct all the graphs of chromatic number >  k. For each  k ∈  N, let us define the class of  k-constructible  graphs recursively k-constructible

as follows:

(i)  Kk  is  k-constructible.

(ii) If  G  is  k-constructible and  x, y ∈ V ( G) are non-adjacent, then also ( G +  xy) /xy  is  k-constructible.

1 This is obvious from the examples of  Kn,n, which are 2-chromatic but whose connectivity and average degree  n  exceeds any constant bound. Which (non-constant) average degree exactly will force the existence of a given subgraph will be the topic of Chapter 7.

2 By Corollaries 5.2.3 and 1.5.4, of course, every graph of sufficiently high chromatic number will contain any given forest.

102

5. Colouring

(iii) If  G 1 , G 2 are  k-constructible and there are vertices  x, y 1 , y 2 such that  G 1  ∩ G 2 =  { x },  xy 1  ∈ E( G 1) and  xy 2  ∈ E( G 2), then also ( G 1  ∪ G 2)  − xy 1  − xy 2 +  y 1 y 2 is  k-constructible (Fig. 5.2.2).

y 1

y 2

y 1

y 2

x

x

Fig. 5.2.2.  The  Haj´

os construction (iii)

One easily checks inductively that all  k-constructible graphs—and hence their supergraphs—are at least  k-chromatic. Indeed, if ( G +  xy) /xy  as in (ii) has a colouring with fewer than  k  colours, then this defines such a colouring also for  G, a contradiction. Similarly, in any colouring of the graph constructed in (iii), the vertices  y 1 and  y 2 do not both have the same colour as  x, so this colouring induces a colouring of either  G 1

or  G 2 and hence uses at least  k  colours.

It is remarkable, though, that the converse holds too:

Theorem 5.2.5. (Hajós 1961)

Let G be a graph and k ∈  N . Then χ( G) >  k if and only if G has a k-constructible subgraph.

Proof . Let  G  be a graph with  χ( G) >  k; we show that  G  has a  k-

constructible subgraph. Suppose not; then  k > 3. Adding some edges if necessary, let us make  G  edge-maximal with the property that none of its subgraphs is  k-constructible. Now  G  is not a complete  r-partite graph for any  r: for then  χ( G) >  k  would imply  r >  k, and  G  would contain the  k-constructible graph  Kk.

Since  G  is not a complete multipartite graph, non-adjacency is not an equivalence relation on  V ( G). So there are vertices  y 1 , x, y 2 such that y 1 x, xy 2

y 1 x, xy 2  /

∈ E( G) but  y 1 y 2  ∈ E( G). Since  G  is edge-maximal without a  k-constructible subgraph, each edge  xyi  lies in some  k-constructible H 1 , H 2

subgraph  Hi  of  G +  xyi ( i = 1 ,  2).

H0

Let  H0

2

2 be an isomorphic copy of  H 2 that contains  x  and  H 2  − H 1

but is otherwise disjoint from  G, together with an isomorphism  v 7→ v0

v0 etc.

from  H 2 to  H0 2 that fixes  H 2  ∩ H0 2 pointwise. Then  H 1  ∩ H0 2 =  { x }, so H := ( H 1  ∪ H0 2)  − xy 1  − xy0 2 +  y 1 y0 2

is  k-constructible by (iii). One vertex at a time, let us identify in  H  each vertex  v0 ∈ H0 −

2

G  with its partner  v; since  vv0  is never an edge of  H,

5.2 Colouring vertices

103

each of these identifications amounts to a construction step of type (ii).

Eventually, we obtain the graph

( H 1  ∪ H 2)  − xy 1  − xy 2 +  y 1 y 2  ⊆ G ; this is the desired  k-constructible subgraph of  G.

¤

5.3 Colouring edges

Clearly, every graph  G  satisfies  χ0( G) > ∆( G). For bipartite graphs, we have equality here:

Proposition 5.3.1. (König 1916)

Every bipartite graph G satisfies χ0( G) = ∆( G) .

Proof . We apply induction on  kGk. For  kGk = 0 the assertion holds.

(1.6.1)

Now assume that  kGk > 1, and that the assertion holds for graphs with fewer edges. Let ∆ := ∆( G), pick an edge  xy ∈ G, and choose a ∆-

∆ , xy

edge-colouring of  G − xy  by the induction hypothesis. Let us refer to the edges coloured  α  as  α-edges, etc.

α-edge

In  G − xy, each of  x  and  y  is incident with at most ∆  −  1 edges.

Hence there are  α, β ∈ {  1 , . . . , ∆  }  such that  x  is not incident with an α, β

α-edge and  y  is not incident with a  β-edge. If  α =  β, we can colour the edge  xy  with this colour and are done; so we may assume that  α 6=  β, and that  x  is incident with a  β-edge.

Let us extend this edge to a maximal walk  W  whose edges are

coloured  β  and  α  alternately. Since no such walk contains a vertex twice (why not?),  W  exists and is a path. Moreover,  W  does not contain  y: if it did, it would end in  y  on an  α-edge (by the choice of  β) and thus have even length, so  W +  xy  would be an odd cycle in  G (cf. Proposition

1.6.1). We now recolour all the edges on  W , swapping  α  with  β. By the choice of  α  and the maximality of  W , adjacent edges of  G − xy  are still coloured differently. We have thus found a ∆-edge-colouring of  G − xy in which neither  x  nor  y  is incident with a  β-edge. Colouring  xy  with  β, we extend this colouring to a ∆-edge-colouring of  G.

¤

Theorem 5.3.2. (Vizing 1964)

Every graph G satisfies

∆( G) 6  χ0( G) 6 ∆( G) + 1  .

Proof . We prove the second inequality by induction on  kGk. For  kGk = 0

V, E

it is trivial. For the induction step let  G = ( V, E) with ∆ := ∆( G)  >  0 be

∆

104

5. Colouring

given, and assume that the assertion holds for graphs with fewer edges.

colouring

Instead of ‘(∆ + 1)-edge-colouring’ let us just say ‘colouring’. An edge α-edge

coloured  α  will again be called an  α-edge.

For every edge  e ∈ G  there exists a colouring of  G − e, by the induction hypothesis. In such a colouring, the edges at a given vertex

v  use at most  d( v) 6 ∆ colours, so some colour  β ∈ {  1 , . . . , ∆ + 1  }  is missing

missing at v. For any other colour  α, there is a unique maximal walk (possibly trivial) starting at  v, whose edges are coloured alternately  α

α/β - path

and  β. This walk is a path; we call it the  α/β - path from v.

Suppose that  G  has no colouring. Then the following holds:

Given xy ∈ E, and any colouring of G − xy in which the

colour α is missing at x and the colour β is missing at y,

(1)

the α/β - path from y ends in x.

Otherwise we could interchange the colours  α  and  β  along this path and colour  xy  with  α, obtaining a colouring of  G (contradiction).

xy 0

Let  xy 0  ∈ G  be an edge. By induction,  G 0 :=  G − xy 0 has a G 0 , c 0 , α

colouring  c 0. Let  α  be a colour missing at  x  in this colouring. Further, y 1 , . . . , yk

let  y 0 , y 1 , . . . , yk  be a maximal sequence of distinct neighbours of  x  in  G, such that  c 0( xyi) is missing in  c 0 at  yi− 1 for each  i = 1 , . . . , k. For each Gi

of the graphs  Gi :=  G − xyi  we define a colouring  ci, setting ½  c

c

0( xyj+1)

for  e =  xyj  with  j ∈ {  0 , . . . , i −  1  }

i( e) :=

ci

c 0( e)

otherwise;

note that in each of these colourings the same colours are missing at  x as in  c 0.

β

Now let  β  be a colour missing at  yk  in  c 0. Clearly,  β  is still missing at  yk  in  ck. If  β  were also missing at  x, we could colour  xyk  with  β

and thus extend  ck  to a colouring of  G. Hence,  x  is incident with a β-edge (in every colouring). By the maximality of  k, therefore, there is an  i ∈ {  1 , . . . , k −  1  }  such that

i

c 0( xyi) =  β .

P

Let  P  be the  α/β - path from  yk  in  Gk (with respect to  ck; Fig. 5.3.1).

By (1),  P  ends in  x, and it does so on a  β-edge, since  α  is missing at  x.

As  β =  c 0( xyi) =  ck( xyi− 1), this is the edge  xyi− 1. In  c 0, however, and hence also in  ci− 1,  β  is missing at  yi− 1 (by (2) and the choice of  yi); let P 0  be the  α/β - path from  yi− 1 in  Gi− 1 (with respect to  ci− 1). Since  P 0

P 0

is uniquely determined, it starts with  yi− 1 P yk; note that the edges of P˚

x  are coloured the same in  ci− 1 as in  ck. But in  c 0, and hence in  ci− 1, there is no  β-edge at  yk (by the choice of  β). Therefore  P 0  ends in  yk, contradicting (1).

¤

5.3 Colouring edges

105

β

yk

α

α

y

P

i+1

β

yi

α

x

β

β

α

yi− 1

y

Gk

0

Fig. 5.3.1.  The  α/β - path  P  in  Gk

Vizing’s theorem divides the finite graphs into two classes according to their chromatic index; graphs satisfying  χ0 = ∆ are called (imagina-tively)  class 1 , those with  χ0 = ∆ + 1 are  class 2 .

5.4 List colouring

In this section, we take a look at a relatively recent generalization of the concepts of colouring studied so far. This generalization may seem a little far-fetched at first glance, but it turns out to supply a fundamental link between the classical (vertex and edge) chromatic numbers of a graph

and its other invariants.

Suppose we are given a graph  G = ( V, E), and for each vertex of G  a list of colours permitted at that particular vertex: when can we colour  G (in the usual sense) so that each vertex receives a colour from its list? More formally, let ( Sv) v∈V  be a family of sets. We call a vertex colouring  c  of  G  with  c( v)  ∈ Sv  for all  v ∈ V  a colouring  from the lists Sv. The graph  G  is called  k-list-colourable, or  k-choosable, if, for  k-choosable every family ( Sv) v∈V  with  |Sv| =  k  for all  v, there is a vertex colouring of  G  from the lists  Sv. The least integer  k  for which  G  is  k-choosable is choice

the  list-chromatic number , or  choice number  ch( G) of  G.

number

ch( G)

List-colourings of edges are defined analogously. The least integer

k  such that  G  has an edge colouring from any family of lists of size  k is the  list-chromatic index  ch 0( G) of  G; formally, we just set ch 0( G) :=

ch 0( G)

ch( L( G)), where  L( G) is the line graph of  G.

In principle, showing that a given graph is  k-choosable is more difficult than proving it to be  k-colourable: the latter is just the special case of the former where all lists are equal to  {  1 , . . . , k }. Thus, ch( G) >  χ( G) and ch 0( G) >  χ0( G) for all graphs  G.

106

5. Colouring

In spite of these inequalities, many of the known upper bounds for

the chromatic number have turned out to be valid for the choice num-

ber, too. Examples for this phenomenon include Brooks’s theorem and

Proposition 5.2.2; in particular, graphs of large choice number still have subgraphs of large minimum degree. On the other hand, it is easy to construct graphs for which the two invariants are wide apart (Exercise 24).

Taken together, these two facts indicate a little how far those general upper bounds on the chromatic number may be from the truth.

The following theorem shows that, in terms of its relationship to

other graph invariants, the choice number differs fundamentally from the chromatic number. As mentioned before, there are 2-chromatic graphs

of arbitrarily large minimum degree, e.g. the graphs  Kn,n. The choice number, however, will be forced up by large values of invariants like  δ,  ε

or  κ:

Theorem 5.4.1. (Alon 1993)

There exists a function f : N  →  N  such that, given any integer k, all graphs G with average degree d( G) >  f ( k)  satisfy  ch( G) >  k.

The proof of Theorem 5.4.1 uses probabilistic methods as introduced in

Chapter 11.

Empirically, the choice number’s different character is highlighted

by another phenomenon: even in cases where known bounds for the

chromatic number could be transferred to the choice number, their proofs have tended to be rather different.

One of the simplest and most impressive examples for this is the list

version of the five colour theorem: every planar graph is 5-choosable.

This had been conjectured for almost 20 years, before Thomassen found

a very simple induction proof. This proof does not use the five colour

theorem—which thus gets reproved in a very different way.

Theorem 5.4.2. (Thomassen 1994)

Every planar graph is 5-choosable.

(4.2.6)

Proof . We shall prove the following assertion for all plane graphs  G  with at least 3 vertices:

Suppose that every inner face of G is bounded by a trian-

gle and its outer face by a cycle C =  v 1  . . . vkv 1 . Suppose further that v 1  has already been coloured with the colour 1, and v 2  has been coloured 2. Suppose finally that

( ∗)

with every other vertex of C a list of at least  3  colours is associated, and with every vertex of G − C a list of at least

5  colours. Then the colouring of v 1  and v 2  can be extended to a colouring of G from the given lists.

5.4 List colouring

107

Let us check first that ( ∗) implies the assertion of the theorem.

Let any plane graph be given, together with a list of 5 colours for each vertex. Add edges to this graph until it is a maximal plane graph  G.

By Proposition 4.2.6,  G  is a plane triangulation; let  v 1 v 2 v 3 v 1 be the boundary of its outer face. We now colour  v 1 and  v 2 (differently) from their lists, and extend this colouring by ( ∗) to a colouring of  G  from the lists given.

Let us now prove ( ∗), by induction on  |G|. If  |G| = 3, then  G =

C  and the assertion is trivial. Now let  |G| > 4, and assume ( ∗) for smaller graphs. If  C  has a chord  vw, then  vw  lies on two unique cycles vw

C 1 , C 2  ⊆ C +  vw  with  v 1 v 2  ∈ C 1 and  v 1 v 2  /

∈ C 2. For  i = 1 ,  2, let  Gi

denote the subgraph of  G  induced by the vertices lying on  Ci  or in its inner face (Fig. 5.4.1). Applying the induction hypothesis first to  G 1

and then—with the colours now assigned to  v  and  w—to  G 2 yields the desired colouring of  G.

v 1

G 1

1

2

v 2 =  w

v

G 2

Fig. 5.4.1.  The induction step with a chord  vw; here the case of  w =  v 2

If  C  has no chord, let  v 1 , u 1 , . . . , um, vk− 1 be the neighbours of  vk  in u 1 , . . . , um

their natural cyclic order order around  vk;3 by definition of  C, all those neighbours  ui  lie in the inner face of  C (Fig. 5.4.2). As the inner faces vk

v 1

vk− 1

P

u 1

v 2

u 3  u 2

C0

Fig. 5.4.2.  The induction step without a chord

3 as in the first proof of the five colour theorem

108

5. Colouring

of  C  are bounded by triangles,  P :=  v 1 u 1  . . . umvk− 1 is a path in  G, and C0

C0 :=  P ∪ ( C − vk) a cycle.

We now choose two different colours  j, ` 6= 1 from the list of  vk  and delete these colours from the lists of all the vertices  ui. Then every list of a vertex on  C0  still has at least 3 colours, so by induction we may colour C0  and its interior, i.e. the graph  G − vk. At least one of the two colours j, `  is not used for  vk− 1, and we may assign that colour to  vk.

¤

As is often the case with induction proofs, the trick of the proof

above lies in the delicately balanced strengthening of the assertion

proved. Note that the proof uses neither traditional colouring arguments (such as swapping colours along a path) nor the Euler formula implicit in the standard proof of the five colour theorem. This suggests that maybe in other unsolved colouring problems too it might be of advantage to

aim straight for their list version, i.e. to prove an assertion of the form ch( G) 6  k  instead of the formally weaker  χ( G) 6  k. Unfortunately, this approach fails for the four colour theorem: planar graphs are  not  in general 4-choosable.

As mentioned before, the chromatic number of a graph and its choice

number may differ a lot. Surprisingly, however, no such examples are

known for edge colourings. Indeed it has been conjectured that none

exist:

List colouring conjecture.  Every graph G satisfies  ch 0( G) =  χ0( G) .

We shall prove the list colouring conjecture for bipartite graphs. As

a tool we shall use orientations of graphs, defined in Chapter 1.10. If  D

N +( v)

is a directed graph and  v ∈ V ( D), we denote by  N +( v) the set, and by d+( v)

d+( v) the number, of vertices  w  such that  D  contains an edge directed from  v  to  w.

To see how orientations come into play in the context of colouring,

let us recall the greedy algorithm from Section 5.2. In order to apply the algorithm to a graph  G, we first have to choose a vertex enumeration v 1 , . . . , vn  of  G. The enumeration chosen defines an orientation of  G: just orient every edge  vivj ‘backwards’, from  vi  to  vj  if  i > j. Then, for each vertex  vi  to be coloured, the algorithm considers only those edges at  vi  that are directed away from  vi: if  d+( v)  < k  for all vertices  v, it will use at most  k  colours. Moreover, the first colour class  U  found by the algorithm has the following property: it is an independent set of vertices to which every other vertex sends an edge. The second colour class has

the same property in  G − U , and so on.

The following lemma generalizes this to orientations  D  of  G  that do not necessarily come from a vertex enumeration, but may contain some

kernel

directed cycles. Let us call an independent set  U ⊆ V ( D) a  kernel  of  D

5.4 List colouring

109

if, for every vertex  v ∈ D − U , there is an edge in  D  directed from  v to a vertex in  U . Note that kernels of non-empty directed graphs are themselves non-empty.

Lemma 5.4.3.  Let H be a graph and ( Sv) v∈V ( H)  a family of lists. If H

has an orientation D with d+( v)  < |Sv| for every v, and such that every induced subgraph of D has a kernel, then H can be coloured from the lists Sv.

Proof . We apply induction on  |H|. For  |H| = 0 we take the empty colouring. For the induction step, let  |H| >  0. Let  α  be a colour occur-α

ring in one of the lists  Sv, and let  D  be an orientation of  H  as stated.

The vertices  v  with  α ∈ Sv  span a non-empty subgraph  D0  in  D; by D0

assumption,  D0  has a kernel  U 6=  ∅.

U

Let us colour the vertices in  U  with  α, and remove  α  from the lists of all the other vertices of  D0. Since each of those vertices sends an edge to  U , the modified lists  S0v  for  v ∈ D − U  again satisfy the condition d+( v)  < |S0 |

v

in  D − U . Since  D − U  is an orientation of  H − U , we can thus colour  H − U  from those lists by the induction hypothesis. As none of these lists contains  α, this extends our colouring  U → { α }  to the desired list colouring of  H.

¤

Theorem 5.4.4. (Galvin 1995)

Every bipartite graph G satisfies  ch 0( G) =  χ0( G) .

Proof . Let  G =: ( X ∪ Y, E), where  { X, Y }  is a vertex bipartition of  G.

X, Y, E

Let us say that two edges of  G meet in X  if they share an end in  X, and correspondingly for  Y . Let  χ0( G) =:  k, and let  c  be a  k-edge-colouring k

of  G.

c

Clearly, ch 0( G) >  k; we prove that ch 0( G) 6  k. Our plan is to use

Lemma 5.4.3 to show that the line graph  H  of  G  is  k-choosable. To apply H

the lemma, it suffices to find an orientation  D  of  H  with  d+( v)  < k  for every vertex  v, and such that every induced subgraph of  D  has a kernel.

To define  D, consider adjacent  e, e0 ∈ E, say with  c( e)  < c( e0). If  e  and D

e0  meet in  X, we orient the edge  ee0 ∈ H  from  e0  towards  e; if  e  and  e0

meet in  Y , we orient it from  e  to  e0 (Fig 5.4.3).

Let us compute  d+( e) for given  e ∈ E =  V ( D). If  c( e) =  i, say, then every  e0 ∈ N +( e) meeting  e  in  X  has its colour in  {  1 , . . . , i −  1  }, and every  e0 ∈ N +( e) meeting  e  in  Y  has its colour in  { i + 1 , . . . , k }.

As any two neighbours  e0  of  e  meeting  e  either both in  X  or both in Y  are themselves adjacent and hence coloured differently, this implies d+( e)  < k  as desired.

It remains to show that every induced subgraph  D0  of  D  has a D0

kernel. We show this by induction on  |D0|. For  D0 =  ∅, the empty set is a kernel; so let  |D0| > 1. Let  E0 :=  V ( D0)  ⊆ E. For every  x ∈ X

E0

at which  E0  has an edge, let  ex ∈ E0  be the edge at  x  with minimum

110

5. Colouring

1

2

1

3

G

2

X

Y

Fig. 5.4.3.  Orienting the line graph of  G

U

c-value, and let  U  denote the set of all those edges  ex. Then every edge e0 ∈ E0  r  U  meets some  e ∈ U  in  X, and the edge  ee0 ∈ D0  is directed from  e0  to  e. If  U  is independent, it is thus a kernel of  D0  and we are home; let us assume, therefore, that  U  is not independent.

e, e0

Let  e, e0 ∈ U  be adjacent, and assume that  c( e)  < c( e0). By definition of  U ,  e  and  e0  meet in  Y , so the edge  ee0 ∈ D0  is directed from  e  to  e0.

By the induction hypothesis,  D0 − e  has a kernel  U 0. If  e0 ∈ U 0, then  U 0

U 0

is also a kernel of  D0, and we are done. If not, there exists an  e00 ∈ U 0

such that  D0  has an edge directed from  e0  to  e00. If  e0  and  e00  met in  X, then  c( e00)  < c( e0) by definition of  D, contradicting  e0 ∈ U . Hence  e0  and e00  meet in  Y , and  c( e0)  < c( e00). Since  e  and  e0  meet in  Y , too, also  e and  e00  meet in  Y , and  c( e)  < c( e0)  < c( e00). So the edge  ee00  is directed from  e  towards  e00, so again  U 0  is also a kernel of  D0.

¤

By Proposition 5.3.1, we now know the exact list-chromatic index of bipartite graphs:

Corollary 5.4.5.  Every bipartite graph G satisfies  ch 0( G) = ∆( G) .

¤

5.5 Perfect graphs

As discussed in Section 5.2, a high chromatic number may occur as a

purely global phenomenon: even when a graph has large girth, and thus

locally looks like a tree, its chromatic number may be arbitrarily high.

Since such ‘global dependence’ is obviously difficult to deal with, one may become interested in graphs where this phenomenon does not occur, i.e.

whose chromatic number is high only when there is a local reason for it.

Before we make this precise, let us note two definitions for a graph  G.

ω( G)

The greatest integer  r  such that  Kr ⊆ G  is the  clique number ω( G) of  G, and the greatest integer  r  such that  Kr ⊆ G (induced) is the  indepen-α( G)

dence number α( G) of  G. Clearly,  α( G) =  ω( G) and  ω( G) =  α( G).

5.5 Perfect graphs

111

A graph is called  perfect  if every induced subgraph  H ⊆ G  has perfect

chromatic number  χ( H) =  ω( H), i.e. if the trivial lower bound of  ω( H) colours always suffices to colour the vertices of  H. Thus, while proving an assertion of the form  χ( G)  > k  may in general be difficult, even in principle, for a given graph  G, it can always be done for a perfect graph simply by exhibiting some  Kk+1 subgraph as a ‘certificate’ for non-colourability with  k  colours.

At first glance, the structure of the class of perfect graphs appears

somewhat contrived: although it is closed under induced subgraphs (if

only by explicit definition), it is not closed under taking general subgraphs or supergraphs, let alone minors (examples?). However, per-

fection is an important notion in graph theory: the fact that several

fundamental classes of graphs are perfect (as if by fluke) may serve as a superficial indication of this.4

What graphs, then, are perfect? Bipartite graphs are, for instance.

Less trivially, the complements of bipartite graphs are perfect, too—

a fact equivalent to K¨

onig’s duality theorem 2.1.1 (Exercise 34). The so-called  comparability graphs  are perfect, and so are the  interval graphs (see the exercises); both these turn up in numerous applications.

In order to study at least one such example in some detail, we

prove here that the chordal graphs are perfect: a graph is  chordal (or chordal

triangulated ) if each of its cycles of length at least 4 has a chord, i.e. if it contains no induced cycles other than triangles.

To show that chordal graphs are perfect, we shall first characterize

their structure. If  G  is a graph with induced subgraphs  G 1,  G 2 and  S, such that  G =  G 1  ∪ G 2 and  S =  G 1  ∩ G 2, we say that  G  arises from  G 1

and  G 2 by  pasting  these graphs together along  S.

pasting

Proposition 5.5.1.  A graph is chordal if and only if it can be con-

[ 12.3.11 ]

structed recursively by pasting along complete subgraphs, starting from complete graphs.

Proof . If  G  is obtained from two chordal graphs  G 1 , G 2 by pasting them together along a complete subgraph, then  G  is clearly again chordal: any induced cycle in  G  lies in either  G 1 or  G 2, and is hence a triangle by assumption. Since complete graphs are chordal, this proves that all

graphs constructible as stated are chordal.

Conversely, let  G  be a chordal graph. We show by induction on  |G|

that  G  can be constructed as described. This is trivial if  G  is complete.

We therefore assume that  G  is not complete, in particular  |G| >  1, and that all smaller chordal graphs are constructible as stated. Let  a, b ∈ G

a, b

4 The class of perfect graphs has duality properties with deep connections to optimization and complexity theory, which are far from understood. Theorem 5.5.5

shows the tip of an iceberg here; for more, the reader is referred to Lovász’s survey cited in the notes.

112

5. Colouring

X

be two non-adjacent vertices, and let  X ⊆ V ( G) r  { a, b }  a minimal C

set of vertices separating  a  from  b. Let  C  denote the component of G 1 , G 2

G − X  containing  a, and put  G 1 :=  G [  V ( C)  ∪ X ] and  G 2 :=  G − C.

Then  G  arises from  G 1 and  G 2 by pasting these graphs together along S

S :=  G [  X ].

Since  G 1 and  G 2 are both chordal (being induced subgraphs of  G) and hence constructible by induction, it suffices to show that  S  is com-s, t

plete. Suppose, then, that  s, t ∈ S  are non-adjacent. By the minimality of  X =  V ( S) as an  a– b  separator, both  s  and  t  have a neighbour in  C.

Hence, there is an  X-path from  s  to  t  in  G 1; we let  P 1 be a shortest such path. Analogously,  G 2 contains a shortest  X-path  P 2 from  s  to  t. But then  P 1  ∪ P 2 is a chordless cycle of length > 4 (Fig. 5.5.1), contradicting our assumption that  G  is chordal.

¤

G 1

G 2

s

P 1

P 2

a

t

b

S

Fig. 5.5.1.  If  G 1 and  G 2 are chordal, then so is  G

Proposition 5.5.2.  Every chordal graph is perfect.

Proof . Since complete graphs are perfect, it suffices by Proposition

5.5.1 to show that any graph  G  obtained from perfect graphs  G 1 , G 2 by pasting them together along a complete subgraph  S  is again perfect. So let  H ⊆ G  be an induced subgraph; we show that  χ( H) 6  ω( H).

Let  Hi :=  H ∩ Gi  for  i = 1 ,  2, and let  T :=  H ∩ S. Then  T  is again complete, and  H  arises from  H 1 and  H 2 by pasting along  T . As an induced subgraph of  Gi, each  Hi  can be coloured with  ω( Hi) colours.

Since  T  is complete and hence coloured injectively, two such colourings, one of  H 1 and one of  H 2, may be combined into a colouring of  H  with max  { ω( H 1) , ω( H 2)  }  6  ω( H) colours—if necessary by permuting the colours in one of the  Hi.

¤

We now come to the main result in the theory of perfect graphs, the

perfect graph theorem:

perfect

graph

Theorem 5.5.3. (Lovász 1972)

theorem

A graph is perfect if and only if its complement is perfect.

5.5 Perfect graphs

113

We shall give two proofs of Theorem 5.5.3. The first of these is Lovász’s original proof, which is still unsurpassed in its clarity and the amount of ‘feel’ for the problem it conveys. Our second proof, due to Gasparian (1996), is in fact a very short and elegant linear algebra proof of another theorem of Lovász’s (Theorem 5.5.5), which easily implies Theorem 5.5.3.

Let us prepare our first proof of the perfect graph theorem by a lemma. Let  G  be a graph and  x ∈ G  a vertex, and let  G0  be obtained from  G  by adding a vertex  x0  and joining it to  x  and all the neighbours of  x. We say that  G0  is obtained from  G  by  expanding  the vertex  x  to an edge  xx0 (Fig. 5.5.2).

expanding

a vertex

x

0

x

G0

G

H

X  r  { x }

Fig. 5.5.2.  Expanding the vertex  x  in the proof of Lemma 5.5.4

Lemma 5.5.4.  Any graph obtained from a perfect graph by expanding a vertex is again perfect.

Proof . We use induction on the order of the perfect graph considered.

Expanding the vertex of  K 1 yields  K 2, which is perfect. For the induction step, let  G  be a non-trivial perfect graph, and let  G0  be obtained from  G  by expanding a vertex  x ∈ G  to an edge  xx0. For our proof that x, x0

G0  is perfect it suffices to show  χ( G0) 6  ω( G0): every proper induced subgraph  H  of  G0  is either isomorphic to an induced subgraph of  G  or obtained from a proper induced subgraph of  G  by expanding  x; in either case,  H  is perfect by assumption and the induction hypothesis, and can hence be coloured with  ω( H) colours.

Let  ω( G) =:  ω ; then  ω( G0)  ∈ { ω, ω + 1  }. If  ω( G0) =  ω + 1, then ω

χ( G0) 6  χ( G) + 1 =  ω + 1 =  ω( G0) and we are done. So let us assume that  ω( G0) =  ω. Then  x  lies in no Kω ⊆ G: together with  x0, this would yield a  Kω+1 in  G0. Let us colour G  with  ω  colours. Since every  Kω ⊆ G  meets the colour class  X  of  x  but X

not  x  itself, the graph  H :=  G − ( X  r  { x }) has clique number  ω( H)  < ω

H

(Fig. 5.5.2). Since  G  is perfect, we may thus colour  H  with  ω −  1 colours.

Now  X  is independent, so the set ( X  r  { x })  ∪ { x0 } =  V ( G0 − H) is also independent. We can therefore extend our ( ω −  1)-colouring of  H  to an ω-colouring of  G0, showing that  χ( G0) 6  ω =  ω( G0) as desired.

¤

114

5. Colouring

Proof of Theorem 5.5.3. Applying induction on  |G|, we show that G = ( V, E)

the complement  G  of any perfect graph  G = ( V, E) is again perfect. For K

|G| = 1 this is trivial, so let  |G| > 2 for the induction step. Let  K  denote α

the set of all vertex sets of complete subgraphs of  G. Put  α( G) =:  α, A

and let  A  be the set of all independent vertex sets  A  in  G  with  |A| =  α.

Every proper induced subgraph of  G  is the complement of a proper induced subgraph of  G, and is hence perfect by induction. For the perfection of  G  it thus suffices to prove  χ( G) 6  ω( G) (=  α). To this end, we shall find a set  K ∈ K  such that  K ∩ A 6=  ∅  for all  A ∈ A; then ω( G − K) =  α( G − K)  < α =  ω( G)  , so by the induction hypothesis

χ( G) 6  χ( G − K) + 1 =  ω( G − K) + 1 6  ω( G) as desired.

Suppose there is no such  K; thus, for every  K ∈ K  there exists a AK

set  AK ∈ A  with  K ∩ AK =  ∅. Let us replace in  G  every vertex  x  by a Gx

complete graph  Gx  of order

¯

¯

k( x)

k( x) := ¯ { K ∈ K | x ∈ A

¯

K } ,

joining all the vertices of  Gx  to all the vertices of  Gy  whenever  x  and  y  are S

G0

adjacent in  G. The graph  G0  thus obtained has vertex set V ( G

x∈V

x),

and two vertices  v ∈ Gx  and  w ∈ Gy  are adjacent in  G0  if and only if x =  y  or  xy ∈ E. Moreover,  G0  can be obtained by repeated vertex expansion from the graph  G [  { x ∈ V | k( x)  >  0  } ]. Being an induced subgraph of  G, this latter graph is perfect by assumption, so  G0  is perfect by Lemma 5.5.4. In particular,

χ( G0) 6  ω( G0)  .

(1)

In order to obtain a contradiction to (1), we now compute in turn the

actual values of  ω( G0) and  χ( G0). By construction of  G0, every maximal S

complete subgraph of  G0  has the form  G0 [

G

x∈X

x ] for some  X ∈ K.

X

So there exists a set  X ∈ K  such that

X

ω( G0) =

k( x)

x∈X

¯

¯

= ¯ { ( x, K) :  x ∈ X, K ∈ K, x ∈ A

¯

K }

X

=

|X ∩ AK|

K ∈K

6  |K| −  1 ;

(2)

5.5 Perfect graphs

115

the last inequality follows from the fact that  |X ∩ AK|  6 1 for all  K

(since  AK  is independent but  G [  X ] is complete), and  |X ∩ AX| = 0 (by the choice of  AX ). On the other hand,

X

|G0| =

k( x)

x∈V

¯

¯

= ¯ { ( x, K) :  x ∈ V, K ∈ K, x ∈ A

¯

K }

X

=

|AK|

K ∈K

=  |K| · α .

As  α( G0) 6  α  by construction of  G0, this implies

|G0|

|G0|

χ( G0) >

>

=  |K| .

(3)

α( G0)

α

Putting (2) and (3) together we obtain

χ( G0) >  |K| > |K| −  1 >  ω( G0)  , a contradiction to (1).

¤

Since the following characterization of perfection is symmetrical in

G  and  G, it clearly implies Theorem 5.5.3. As our proof of Theorem 5.5.5 will again be from first principles, we thus obtain a second and

independent proof of the perfect graph theorem.

Theorem 5.5.5. (Lovász 1972)

A graph G is perfect if and only if

|H|  6  α( H)  · ω( H)

( ∗)

for all induced subgraphs H ⊆ G.

Proof . Let us write  V ( G) =:  V =:  { v 1 , . . . , vn }, and put  α :=  α( G) V, vi, n

and  ω :=  ω( G). The necessity of ( ∗) is immediate: if  G  is perfect, then α, ω

every induced subgraph  H  of  G  can be partitioned into at most  ω( H) colour classes each containing at most  α( H) vertices, and ( ∗) follows.

To prove sufficiency, we apply induction on  n =  |G|. Assume that every induced subgraph  H  of  G  satisfies ( ∗), and suppose that  G  is not perfect. By the induction hypothesis, every proper induced subgraph of

G  is perfect. Hence, every non-empty independent set  U ⊆ V  satisfies χ( G − U ) =  ω( G − U ) =  ω .

(1)

116

5. Colouring

Indeed, while the first equality is immediate from the perfection of  G − U , the second is easy: ‘6’ is obvious, while  χ( G − U )  < ω  would imply χ( G) 6  ω, so  G  would be perfect contrary to our assumption.

Let us apply (1) to a singleton  U =  { u }  and consider an  ω-colouring of  G − u. Let  K  be the vertex set of any  Kω  in  G. Clearly, if u /

∈ K then K meets every colour class of G − u;

(2)

if u ∈ K then K meets all but exactly one colour class of G − u. (3) A 0

Let  A 0 =  { u 1 , . . . , uα }  be an independent set in  G  of size  α.

Let  A 1 , . . . , Aω  be the colour classes of an  ω-colouring of  G − u 1, let Aω+1 , . . . , A 2 ω  be the colour classes of an  ω-colouring of  G − u 2, and Ai

so on; altogether, this gives us  αω + 1 independent sets  A 0 , A 1 , . . . , Aαω

in  G. For each  i = 0 , . . . , αω, there exists by (1) a  Kω ⊆ G − Ai; we Ki

denote its vertex set by  Ki.

Note that if  K  is the vertex set of any  Kω  in  G, then K ∩ Ai =  ∅ for exactly one i ∈ {  0 , . . . , αω + 1  }.

(4)

Indeed, if  K ∩ A 0 =  ∅  then  K ∩ Ai 6=  ∅  for all  i 6= 0, by definition of  Ai and (2). Similarly if  K ∩ A 0  6=  ∅, then  |K ∩ A 0 | = 1, so  K ∩ Ai =  ∅  for exactly one  i 6= 0: apply (3) to the unique vertex  u ∈ K ∩ A 0, and (2) to all the other vertices  u ∈ A 0.

J

Let  J  be the real ( αω + 1)  × ( αω + 1) matrix with zero entries in A

the main diagonal and all other entries 1. Let  A  be the real ( αω + 1)  × n matrix whose rows are the incidence vectors of the subsets  Ai ⊆ V : if ai 1 , . . . , ain  denote the entries of the  i th row of  A, then  aij = 1 if  vj ∈ Ai, B

and  aij = 0 otherwise. Similarly, let  B  denote the real  n × ( αω + 1) matrix whose columns are the incidence vectors of the subsets  Ki ⊆ V .

Now while  |Ki ∩ Ai| = 0 for all  i  by the choice of  Ki, we have  Ki ∩ Aj 6=  ∅

and hence  |Ki ∩ Aj| = 1 whenever  i 6=  j, by (4). Thus, AB =  J.

Since  J  is non-singular, this implies that  A  has rank  αω + 1. In particular,  n >  αω + 1, which contradicts ( ∗) for  H :=  G.

¤

By definition, every induced subgraph of a perfect graph is again

perfect. The property of perfection can therefore be characterized by

forbidden induced subgraphs: there exists a set  H  of imperfect graphs such that any graph is perfect if and only if it has no induced subgraph isomorphic to an element of  H. (For example, we may choose as  H  the set of all imperfect graphs with vertices in N.)

Naturally, it would be desirable to keep  H  as small as possible. In fact, one of the best known conjectures in graph theory says that  H

5.5 Perfect graphs

117

need only contain two types of graph: the odd cycles of length > 5 and their complements. (Neither of these are perfect—why?) Or, rephrased

slightly:

Perfect Graph Conjecture. (Berge 1966)

A graph G is perfect if and only if neither G nor G contains an odd cycle of length at least  5  as an induced subgraph.

Clearly, this conjecture implies the perfect graph theorem. In fact, that theorem had also been conjectured by Berge: until its proof, it was

known as the ‘weak’ version of the perfect graph conjecture, the above

conjecture being the ‘strong’ version.

Graphs  G  such that neither  G  nor  G  contains an induced odd cycle of length at least 5 have been called  Berge graphs. Thus all perfect graphs are Berge graphs, and the perfect graph conjecture claims that all Berge graphs are perfect. This has been approximately verified by Prömel & Steger (1992), who proved that the proportion of perfect graphs to Berge graphs on  n  vertices tends to 1 as  n → ∞.

Exercises

1.  −  Show that the four colour theorem does indeed solve the map colouring problem stated in the first sentence of the chapter. Conversely, does

the 4-colourability of every map imply the four colour theorem?

2.  −  Show that, for the map colouring problem above, it suffices to consider maps such that no point lies on the boundary of more than three

countries. How does this affect the proof of the four colour theorem?

3.

Try to turn the proof of the five colour theorem into one of the four

colour theorem, as follows. Defining  v  and  H  as before, assume inductively that  H  has a 4-colouring; then proceed as before. Where does the proof fail?

4.

Calculate the chromatic number of a graph in terms of the chromatic

numbers of its blocks.

5.  −  Show that every graph  G  has a vertex ordering for which the greedy algorithm uses only  χ( G) colours.

6.

For every  n >  1, find a bipartite graph on 2 n  vertices, ordered in such a way that the greedy algorithm uses  n  rather than 2 colours.

7.

Consider the following approach to vertex colouring. First, find a max-

imal independent set of vertices and colour these with colour 1; then

find a maximal independent set of vertices in the remaining graph and

colour those 2, and so on. Compare this algorithm with the greedy

algorithm: which is better?

118

5. Colouring

8.

Show that the bound of Proposition 5.2.2 is always at least as sharp as that of Proposition 5.2.1.

9.

Find a function  f  such that every graph of arboricity at least  f ( k) has colouring number at least  k, and a function  g  such that every graph of colouring number at least  g( k) has arboricity at least  k, for all  k ∈  N.

(The  arboricity  of a graph is defined in Chapter 3.5.)

10.  −  A  k-chromatic graph is called  critically k-chromatic, or just  critical , if  χ( G − v)  < k  for every  v ∈ V ( G). Show that every  k-chromatic graph has a critical  k-chromatic induced subgraph, and that any such subgraph has minimum degree at least  k −  1.

11.

Determine the critical 3-chromatic graphs.

12.+ Show that every critical  k-chromatic graph is ( k −  1) - edge-connected.

13.

Given  k ∈  N, find a constant  ck >  0 such that every graph  G  with

|G| > 3 k  and  α( G) 6  k  contains a cycle of length at least  ck |G|.

14.  −  Find a graph  G  for which Brooks’s theorem yields a significantly weaker bound on  χ( G) than Proposition 5.2.2.

15.+ Show that, in order to prove Brooks’s theorem for a graph  G = ( V, E), we may assume that  κ( G) > 2 and ∆( G) > 3. Prove the theorem under these assumptions, showing first the following two lemmas.

(i) Let  v 1 , . . . , vn  be an enumeration of  V . If every  vi ( i < n) has a neighbour  vj  with  j > i, and if  v 1 vn, v 2 vn ∈ E  but  v 1 v 2  /

∈ E,

then the greedy algorithm uses at most ∆( G) colours.

(ii) If  G  is not complete and  vn  has maximum degree in  G, then  vn has neighbours  v 1 , v 2 as in (i).

16.

Given a graph  G  and  k ∈  N, let  PG( k) denote the number of vertex colourings  V ( G)  → {  1 , . . . , k }. Show that  PG  is a polynomial in  k  of degree  n :=  |G|, in which the coefficient of  kn  is 1 and the coefficient of  kn− 1 is  −kGk. ( PG  is called the  chromatic polynomial  of  G.) (Hint. Apply induction on  kGk. In the induction step, compare the values of  PG( k),  PG−e( k) and  PG/e( k).) 17.+ Determine the class of all graphs  G  for which  PG( k) =  k ( k −  1) n− 1. (As in the previous exercise, let  n :=  |G|, and let  PG  denote the chromatic polynomial of  G.)

18.

In the definition of  k-constructible graphs, replace the axiom (ii) by (ii) 0 Every supergraph of a k-constructible graph is k-constructible; and the axiom (iii) by

(iii) 0 If G is a graph with vertices x, y 1 , y 2  such that y 1 y 2  ∈ E( G) but xy 1 , xy 2  /

∈ E( G) , and if both G +  xy 1  and G +  xy 2  are k-constructible, then G is k-constructible.

Show that a graph is  k-constructible with respect to this new definition if and only if its chromatic number is at least  k.

Exercises

119

19.  −  An  n × n - matrix with entries from  {  1 , . . . , n }  is called a  Latin square if every element of  {  1 , . . . , n }  appears exactly once in each column and exactly once in each row. Recast the problem of constructing Latin

squares as a colouring problem.

20.

Without using Proposition 5.3.1, show that  χ0( G) =  k  for every  k-

regular bipartite graph  G.

21.

Prove Proposition 5.3.1 from the statement of the previous exercise.

22.+ For every  k ∈  N, construct a triangle-free  k-chromatic graph.

23.  −  Without using Theorem 5.4.2, show that every plane graph is 6-list-colourable.

24.

For every integer  k, find a 2-chromatic graph whose choice number is at least  k.

25.  −  Find a general upper bound for ch 0( G) in terms of  χ0( G).

26.

Compare the choice number of a graph with its colouring number:

which is greater? Can you prove the analogue of Theorem 5.4.1 for the colouring number?

27.+ Prove that the choice number of  Kr 2 is  r.

28.

The  total chromatic number χ00( G) of a graph  G = ( V, E) is the least number of colours needed to colour the vertices and edges of  G  simultaneously so that any adjacent or incident elements of  V ∪ E  are coloured differently. The  total colouring conjecture  says that  χ00( G) 6 ∆( G) + 2.

Bound the total chromatic number from above in terms of the list-

chromatic index, and use this bound to deduce a weakening of the

total colouring conjecture from the list colouring conjecture.

29.  −  Find a directed graph that has no kernel.

30.+ Prove  Richardson’s theorem: every directed graph without odd directed cycles has a kernel.

31.

Show that every bipartite planar graph is 3-list-colourable.

(Hint. Apply the previous exercise and Lemma 5.4.3.)

32.  −  Show that perfection is closed neither under edge deletion nor under edge contraction.

33.  −  Deduce Theorem 5.5.5 from the perfect graph conjecture.

34.

Use König’s Theorem 2.1.1 to show that the complement of any bipartite graph is perfect.

35.

Using the results of this chapter, find a one-line proof of the following theorem of König, the dual of Theorem 2.1.1: in any bipartite graph without isolated vertices, the minimum number of edges meeting all

vertices equals the maximum number of independent vertices.

36.

A graph is called a  comparability graph  if there exists a partial ordering of its vertex set such that two vertices are adjacent if and only if they are comparable. Show that every comparability graph is perfect.

120

5. Colouring

37.

A graph  G  is called an  interval graph  if there exists a set  { Iv | v ∈ V ( G)  }

of real intervals such that  Iu ∩ Iv 6=  ∅  if and only if  uv ∈ E( G).

(i) Show that every interval graph is chordal.

(ii) Show that the complement of any interval graph is a compara-

bility graph.

(Conversely, a chordal graph is an interval graph if its complement is a comparability graph; this is a theorem of Gilmore and Hoffman (1964).)

38.

Show that  χ( H)  ∈ { ω( H)  , ω( H) + 1  }  for every line graph  H.

39.+ Characterize the graphs whose line graphs are perfect.

40.

Show that a graph  G  is perfect if and only if every non-empty induced subgraph  H  of  G  contains an independent set  A ⊆ V ( H) such that ω( H − A)  < ω( H).

41.+ Consider the graphs  G  for which every induced subgraph  H  has the property that every maximal complete subgraph of  H  meets every maximal independent vertex set in  H.

(i) Show that these graphs  G  are perfect.

(ii) Show that these graphs  G  are precisely the graphs not containing an induced copy of  P  3.

42.+ Show that in every perfect graph  G  one can find a set  A  of independent vertex sets and a set  O  of vertex sets of complete subgraphs such that S

S

A =  V ( G) =

O  and every set in  A  meets every set in  O.

(Hint. Lemma 5.5.4.)

43.+ Let  G  be a perfect graph. As in the proof of Theorem 5.5.3, replace every vertex  x  of  G  with a perfect graph  Gx (not necessarily complete).

Show that the resulting graph  G0  is again perfect.

44.

Let  H 1 and  H 2 be two sets of imperfect graphs, each minimal with the property that a graph is perfect if and only if it has no induced

subgraph in  Hi ( i = 1 ,  2). Do  H 1 and  H 2 contain the same graphs, up to isomorphism?

Notes

The authoritative reference work on all questions of graph colouring is T.R.

Jensen & B. Toft,  Graph Coloring Problems, Wiley 1995. Starting with a brief survey of the most important results and areas of research in the field, this monograph gives a detailed account of over 200 open colouring problems, complete with extensive background surveys and references. Most of the remarks below are discussed comprehensively in this book, and all the references for this chapter can be found there.

The  four colour problem, whether every map can be coloured with four colours so that adjacent countries are shown in different colours, was raised by a certain Francis Guthrie in 1852. He put the question to his brother Frederick, who was then a mathematics undergraduate in Cambridge. The problem was

Notes

121

first brought to the attention of a wider public when Cayley presented it to the London Mathematical Society in 1878. A year later, Kempe published

an incorrect proof, which was in 1890 modified by Heawood into a proof of the five colour theorem. In 1880, Tait announced ‘further proofs’ of the four colour conjecture, which never materialized; see the notes for Chapter 10.

The first generally accepted proof of the four colour theorem was published by Appel and Haken in 1977. The proof builds on ideas that can be traced back as far as Kempe’s paper, and were developed largely by Birkhoff and Heesch. Very roughly, the proof sets out first to show that every plane triangulation must contain at least one of 1482 certain ‘unavoidable configurations’. In a second step, a computer is used to show that each of those configurations is ‘reducible’, i.e., that any plane triangulation containing such a configuration can be 4-coloured by piecing together 4-colourings of smaller plane triangulations. Taken together, these two steps amount to an inductive proof that all plane triangulations, and hence all planar graphs, can be 4-coloured.

Appel & Haken’s proof has not been immune to criticism, not only because of their use of a computer. The authors responded with a 741 page long algorithmic version of their proof, which addresses the various criticisms and corrects a number of errors (e.g. by adding more configurations to the

‘unavoidable’ list): K. Appel & W. Haken,  Every Planar Map is Four Col-orable, American Mathematical Society 1989. A much shorter proof, which is based on the same ideas (and, in particular, uses a computer in the same way) but can be more readily verified both in its verbal and its computer part, has been given by N. Robertson, D. Sanders, P.D. Seymour & R. Thomas, The

four-colour theorem,  J. Combin. Theory B 70 (1997), 2–44.

A relatively short proof of Gr¨

otzsch’s theorem was found by C. Thomassen,

Grötzsch’s 3-color theorem and its counterparts for the torus and the projective plane,  J. Combin. Theory B 62 (1994), 268–279. Although not touched upon in this chapter, colouring problems for graphs embedded in surfaces other than the plane form a substantial and interesting part of colouring theory; see B. Mohar & C. Thomassen,  Graphs on Surfaces, Johns Hopkins University Press, to appear.

The proof of Brooks’s theorem indicated in Exercise 15, where the greedy algorithm is applied to a carefully chosen vertex ordering, is due to Lovász (1973). Lovász (1968) was also the first to  construct  graphs of arbitrarily large girth and chromatic number, graphs whose existence Erd˝

os had proved

by probabilistic methods ten years earlier.

A. Urquhart, The graph constructions of Hajós and Ore,  J. Graph Theory 26 (1997), 211–215, showed that not only do the graphs of chromatic number at least  k  each  contain  a  k-constructible graph (as by Haj´

os’s theorem); they

are in fact all themselves  k-constructible. Algebraic tools for showing that the chromatic number of a graph is large have been developed by Kleitman & Lovász (1982), and by Alon & Tarsi (1992); see Alon’s paper cited below.

List colourings were first introduced in 1976 by Vizing. Among other

things, Vizing proved the list-colouring equivalent of Brooks’s theorem. Voigt (1993) constructed a plane graph of order 238 that is not 4-choosable; thus, Thomassen’s list version of the five colour theorem is best possible. A stimulating survey on the list-chromatic number and how it relates to the more

122

5. Colouring

classical graph invariants (including a proof of Theorem 5.4.1) is given by N. Alon, Restricted colorings of graphs, in (K. Walker, ed.)  Surveys in Combinatorics, LMS Lecture Notes 187, Cambridge University Press 1993. Both the

list colouring conjecture and Galvin’s proof of the bipartite case are originally stated for multigraphs. Kahn (1994) proved that the conjecture is asymptot-ically correct, as follows: given any  ² >  0, every graph  G  with large enough maximum degree satisfies ch 0( G) 6 (1 +  ²)∆( G).

The total colouring conjecture was proposed around 1965 by Vizing and by Behzad; see Jensen & Toft for details.

A gentle introduction to the basic facts about perfect graphs and their applications is given by M.C. Golumbic,  Algorithmic Graph Theory and Perfect Graphs, Academic Press 1980. Our first proof of the perfect graph theorem

follows L. Lovász’s survey on perfect graphs in (L.W. Beineke and R.J. Wilson, eds.)  Selected Topics in Graph Theory 2, Academic Press 1983. The theorem was also proved independently, and only a little later, by Fulkerson. Our second proof, the proof of Theorem 5.5.5, is due to G.S. Gasparian, Minimal imperfect graphs: a simple approach,  Combinatorica 16 (1996), 209–212. The approximate proof of the perfect graph conjecture is due to H.J. Prömel & A. Steger, Almost all Berge graphs are perfect,  Combinatorics, Probability and Computing 1 (1992), 53–79.

6

Flows

Let us view a graph as a network: its edges carry some kind of flow—of

water, electricity, data or similar. How could we model this precisely?

For a start, we ought to know how much flow passes through each

edge  e =  xy, and in which direction. In our model, we could assign a positive integer  k  to the pair ( x, y) to express that a flow of  k  units passes through  e  from  x  to  y, or assign  −k  to ( x, y) to express that  k units of flow pass through  e  the other way, from  y  to  x. For such an assignment  f :  V  2  →  Z we would thus have  f ( x, y) =  −f ( y, x) whenever x  and  y  are adjacent vertices of  G.

Typically, a network will have only a few nodes where flow enters

or leaves the network; at all other nodes, the total amount of flow into that node will equal the total amount of flow out of it. For our model

this means that, at most nodes  x, the function  f  will satisfy  Kirchhoff ’s law

X

Kirchhoff’s

f ( x, y) = 0  .

law

y ∈N ( x)

In this chapter, we call any map  f :  V  2  →  Z with the above two properties a ‘flow’ on  G. Sometimes, we shall replace Z with another group, and as a rule we consider multigraphs rather than graphs.1 As

it turns out, the theory of those ‘flows’ is not only useful as a model for real flows: it blends so well with other parts of graph theory that some deep and surprising connections become visible, connections particularly with connectivity and colouring problems.

1 For consistency, we shall phrase some of our proposition for graphs only: those whose proofs rely on assertions proved (for graphs) earlier in the book. However, all those results remain true for multigraphs.

124

6. Flows

6.1 Circulations

In the context of flows, we have to be able to speak about the ‘directions’

G = ( V, E)

of an edge. Since, in a multigraph  G = ( V, E), an edge  e =  xy  is not identified uniquely by the pair ( x, y) or ( y, x), we define directed edges as triples:

→

→

E

E :=  { ( e, x, y)  | e ∈ E;  x, y ∈ V ;  e =  xy } .

direction

Thus, an edge  e =  xy  with  x 6=  y  has the two  directions ( e, x, y) and ( e, x, y)

( e, y, x); a loop  e =  xx  has only one direction, the triple ( e, x, x). For

→

←

e

given  →

e = ( e, x, y)  ∈ E, we set  ←

e := ( e, y, x), and for an arbitrary set

→

F ⊆ →

E  of edge directions we put

←

→

←

F

F :=  { ←

e | →

e ∈ F } .

→

←

→

→

Note that  E  itself is symmetrical:  E =  E. For  X, Y ⊆ V  and  F ⊆ →

E,

define

→

→

→

F ( X, Y )

F ( X, Y ) :=  { ( e, x, y)  ∈ F | x ∈ X;  y ∈ Y ;  x 6=  y } ,

→

→

→

F ( x, Y )

abbreviate  F ( { x }, Y ) to  F ( x, Y ) etc., and write

→

→

→

→

F ( x)

F ( x) :=  F ( x, V ) =  F ( { x }, { x })  .

X

Here, as below,  X  denotes the complement  V  r  X  of a vertex set  X ⊆ V.

Note that any loops at vertices  x ∈ X ∩ Y  are disregarded in the defini-

→

→

tions of  F ( X, Y ) and  F ( x).

0

Let  H  be an abelian semigroup,2 written additively with zero 0.

→

f

Given vertex sets  X, Y ⊆ V  and a function  f :  E → H, let X

f ( X, Y )

f ( X, Y ) :=

f ( →

e)  .

~

e ∈ ~

E ( X,Y )

f ( x, Y )

Instead of  f ( { x }, Y ) we again write  f ( x, Y ), etc.

circulation

From now on, we assume that  H  is a group. We call  f  a  circulation on  G (with values in  H), or an  H-circulation, if  f  satisfies the following two conditions:

→

(F1)  f ( e, x, y) =  −f ( e, y, x) for all ( e, x, y)  ∈ E  with  x 6=  y; (F2)  f ( v, V ) = 0 for all  v ∈ V .

2 This chapter contains no group theory. The only semigroups we ever consider for  H  are the natural numbers, the integers, the reals, the cyclic groups Z k, and (once) the Klein four-group.

6.1 Circulations

125

If  f  satisfies (F1), then

f ( X, X) = 0

for all  X ⊆ V . If  f  satisfies (F2), then

X

f ( X, V ) =

f ( x, V ) = 0  .

x∈X

Together, these two basic observations imply that, in a circulation, the net flow across any cut is zero:

[ 6.3.1 ]

Proposition 6.1.1.  If f is a circulation, then f ( X, X) = 0  for every

[ 6.5.2 ]

[ 6.6.1 ]

set X ⊆ V .

Proof .  f ( X, X) =  f ( X, V )  − f ( X, X) = 0  −  0 = 0.

¤

Since bridges form cuts by themselves, Proposition 6.1.1 implies

that circulations are always zero on bridges:

Corollary 6.1.2.  If f is a circulation and e =  xy is a bridge in G, then f ( e, x, y) = 0 .

¤

6.2 Flows in networks

In this section we give a brief introduction to the kind of network flow theory that is now a standard proof technique in areas such as matching and connectivity. By way of example, we shall prove a classic result of this theory, the so-called  max-flow min-cut  theorem of Ford and Fulkerson. This theorem alone implies Menger’s theorem without much dif-ficulty (Exercise 3), which indicates some of the natural power lying in this approach.

Consider the task of modelling a network with one source  s  and one sink  t, in which the amount of flow through a given link between two nodes is subject to a certain capacity of that link. Our aim is to

determine the maximum net amount of flow through the network from

s  to  t. Somehow, this will depend both on the structure of the network and on the various capacities of its connections—how exactly, is what

we wish to find out.

Let  G = ( V, E) be a multigraph,  s, t ∈ V  two fixed vertices, and G = ( V, E)

→

c:  E →  N a map; we call  c  a  capacity function  on  G, and the tuple s, t, c, N

N := ( G, s, t, c) a  network . Note that  c  is defined independently for network

→

the two directions of an edge. A function  f :  E →  R is a  flow  in  N  if it flow

satisfies the following three conditions (Fig. 6.2.1):

126

6. Flows

→

(F1)  f ( e, x, y) =  −f ( e, y, x) for all ( e, x, y)  ∈ E  with  x 6=  y; (F2 0)  f ( v, V ) = 0 for all  v ∈ V  r  { s, t };

→

(F3)  f ( →

e) 6  c( →

e) for all  →

e ∈ E.

integral

We call  f integral  if all its values are integers.

1

1

3

1

t

2

s

3

1

0

2

Fig. 6.2.1.  A network flow in short notation: all values refer to the direction indicated (capacities are not shown)

f

Let  f  be a flow in  N . If  S ⊆ V  is such that  s ∈ S  and  t ∈ S, we call cut in N

the pair ( S, S) a  cut in N , and  c( S, S) the  capacity  of this cut.

capacity

Since  f  now has to satisfy only (F2 0) rather than (F2), we no longer have  f ( X, X) = 0 for all  X ⊆ V (as in Proposition 6.1.1). However, the value is the same for all cuts:

Proposition 6.2.1.  Every cut ( S, S)  in N satisfies f ( S, S) =  f ( s, V ) .

Proof . As in the proof of Proposition 6.1.1, we have f ( S, S) =  f ( S, V )  − f ( S, S) X

=  f ( s, V ) +

f ( v, V )  −  0

(F1)

v ∈S r { s }

=  f ( s, V )  .

(F2 0)

¤

The common value of  f ( S, S) in Proposition 6.2.1 will be called the  total total value

value  of  f  and denoted by  |f |;3 the flow shown in Figure 6.2.1 has total

|f|

value 3.

By (F3), we have

|f| =  f( S, S) 6  c( S, S)

for every cut ( S, S) in  N . Hence the total value of a flow in  N  is never larger than the smallest capacity of a cut. The following  max-flow min-cut  theorem states that this upper bound is always attained by some flow:

3 Thus, formally,  |f|  may be negative. In practice, however, we can change the sign of  |f |  simply by swapping the roles of  s  and  t.

6.2 Flows in networks

127

Theorem 6.2.2. (Ford & Fulkerson 1956)

max-flow

In every network, the maximum total value of a flow equals the minimum min-cut

theorem

capacity of a cut.

Proof . Let  N = ( G, s, t, c) be a network, and  G =: ( V, E). We shall define a sequence  f 0 , f 1 , f 2 , . . .  of integral flows in  N  of strictly increasing total value, i.e. with

|f 0 | < |f 1 | < |f 2 | < . . .

Clearly, the total value of an integral flow is again an integer, so in fact

|fn+1 | >  |fn| + 1 for all  n. Since all these numbers are bounded above by the capacity of any cut in  N , our sequence will terminate with some flow  fn. Corresponding to this flow, we shall find a cut of capacity cn =  |fn|. Since no flow can have a total value greater than  cn, and no cut can have a capacity less than  |fn|, this number is simultaneously the maximum and the minimum referred to in the theorem.

→

For  f 0, we set  f 0( →

e) := 0 for all  →

e ∈ E. Having defined an integral

flow  fn  in  N  for some  n ∈  N, we denote by  Sn  the set of all vertices  v Sn

such that  G  contains an  s– v  walk  x 0 e 0  . . . è− 1 x`  with fn(  →

ei)  < c(  →

ei)

for all  i < `; here,  →

ei := ( ei, xi, xi+1) (and, of course,  x 0 =  s  and  x` =  v).

If  t ∈ Sn, let  W =  x 0 e 0  . . . è− 1 x`  be the corresponding  s– t  walk; W

without loss of generality we may assume that  W  does not repeat any vertices. Let

² := min  { c(  →

ei)  − fn(  →

ei)  | i < ` } .

²

Then  ² >  0, and since  fn (like  c) is integral by assumption,  ²  is an integer.

Let





  fn( →e) +  ²  for  →e =  →

ei,  i = 0 , . . . , ` −  1;

fn+1:  →

e 7→   fn( →e) −²  for  →e =  ←ei,  i = 0 ,...,`− 1;

  fn( →e)

for  e /

∈ W .

Intuitively,  fn+1 is obtained from  fn  by sending additional flow of value  ²

along  W  from  s  to  t (Fig. 6.2.2).

1

1

1

3

t

W

2

s

3

1

0

2

Fig. 6.2.2.  An ‘augmenting path’  W  with increment  ² = 2, for constant flow  fn = 0 and capacities  c = 3

128

6. Flows

Clearly,  fn+1 is again an integral flow in  N . Let us compute its total value  |fn+1 | =  fn+1( s, V ). Since  W  contains the vertex  s  only once,  →

e 0

is the only triple ( e, x, y) with  x =  s  and  y ∈ V  whose  f -value was changed. This value, and hence that of  fn+1( s, V ) was raised. Therefore

|fn+1 | > |fn|  as desired.

If  t /

∈ Sn, then ( Sn, Sn) is a cut in  N . By (F3) for  fn, and the definition of  Sn, we have

fn( →

e) =  c( →

e)

→

for all  →

e ∈ E( Sn, Sn), so

|fn| =  fn( Sn, Sn) =  c( Sn, Sn)

as desired.

¤

Since the flow constructed in the proof of Theorem 6.2.2 is integral, we have also proved the following:

Corollary 6.2.3.  In every network (with integral capacity function) there exists an integral flow of maximum total value.

¤

6.3 Group-valued flows

Let  G = ( V, E) be a multigraph and  H  an abelian group. If  f  and f +  g

g  are two  H-circulations then, clearly, ( f +  g):  →

e 7→ f ( →

e) +  g( →

e) and

−f

−f:  →e 7→ −f( →e) are again  H-circulations. The  H-circulations on  G  thus form a group in a natural way.

→

→

nowhere

A function  f :  E → H  is  nowhere zero  if  f ( →

e)  6= 0 for all  →

e ∈ E. An

zero

H-circulation that is nowhere zero is called an  H-flow .4 Note that the H-flow

set of  H-flows on  G  is not closed under addition: if two  H-flows add up to zero on some edge  →

e, then their sum is no longer an  H-flow. By

Corollary 6.1.2, a graph with an  H-flow cannot have a bridge.

For finite groups  H, the number of  H-flows on  G—and, in particular, their existence—surprisingly depends only on the order of  H, not on  H

itself:

Theorem 6.3.1. (Tutte 1954)

For every multigraph G there exists a polynomial P such that, for any

¡

¢

finite abelian group H, the number of H-flows on G is P |H| −  1  .

4 This terminology seems simplest for our purposes but is not standard; see the notes.

6.3 Group-valued flows

129

Proof . Let  G =: ( V, E); we use induction on  m :=  |E|. Let us assume

(6.1.1)

first that all the edges of  G  are loops. Then, given any finite abelian

→

group  H, every map  E → H  r  {  0  }  is an  H-flow on  G. Since  | →

E| =  |E|

¡

¢ m

when all edges are loops, there are  |H| −  1

such maps, and  P :=  xm

is the polynomial sought.

Now assume there is an edge  e 0 =  xy ∈ E  that is not a loop; let e 0 =  xy

→

e 0 := ( e 0 , x, y) and  E0 :=  E  r  { e 0  }. We consider the multigraphs E0

G 1 :=  G − e 0 and  G 2 :=  G/e 0  .

By the induction hypothesis, there are polynomials  Pi  for  i = 1 ,  2 such P 1 , P 2

that, for any finite abelian group  H  and  k :=  |H| −  1, the number of k

H-flows on  Gi  is  Pi( k). We shall prove that the number of  H-flows on G  equals  P 2( k)  − P 1( k); then  P :=  P 2  − P 1 is the desired polynomial.

Let  H  be given, and denote the set of all  H-flows on  G  by  F . We H

are trying to show that

F

|F | =  P 2( k)  − P 1( k)  .

(1)

→

The  H-flows on  G 1 are precisely the restrictions to  E0  of those  H-circulations on  G  that are zero on  e 0 but nowhere else. Let us denote the set of these circulations on  G  by  F 1; then

F 1

P 1( k) =  |F 1 | .

Our aim is to show that, likewise, the  H-flows on  G 2 correspond bijectively to those  H-circulations on  G  that are nowhere zero except possibly on  e 0. The set  F 2 of those circulations on  G  then satisfies F 2

P 2( k) =  |F 2 | ,

and  F 2 is the disjoint union of  F 1 and  F . This will prove (1), and hence the theorem.

E0( x, y)

v

x

e

0

0

y

G

G 2

Fig. 6.3.1.  Contracting the edge  e 0

In  G 2, let  v 0 :=  ve  be the vertex contracted from  e 0

0 (Fig. 6.3.1;

v 0

see Chapter 1.10). We are looking for a bijection  f 7→ g  between  F 2

130

6. Flows

and the set of  H-flows on  G 2. Given  f , let  g  be the restriction of  f  to

→

E0  r  →

E0( y, x). (As the  x– y  edges  e ∈ E0  become loops in  G 2, they have on-ly the one direction ( e, v 0 , v 0) there; as its  g-value, we choose  f ( e, x, y).) Then  g  is indeed an  H-flow on  G 2; note that (F2) holds at  v 0 by Proposition 6.1.1 for  G, with  X :=  { x, y }.

It remains to show that the map  f 7→ g  is a bijection. If we are given an  H-flow  g  on  G 2 and try to find an  f ∈ F 2 with  f 7→ g, then  f ( →

e) is

→

already determined as  f ( →

e) =  g( →

e) for all  →

e ∈ E0  r  →

E0( y, x); by (F1), we

→

further have  f ( →

e) =  −f ( ←

e) for all  →

e ∈ E0( y, x). Thus our map  f 7→ g  is

bijective if and only if for given  g  there is always a unique way to define the remaining values of  f (  →

e 0) and  f (  ←

e 0) so that  f  satisfies (F1) in  e 0 and

(F2) in  x  and  y.

V 0

This is indeed the case. Let  V 0 :=  V  r  { x, y }. As  g  satisfies (F2), the  f -values fixed already are such that

f ( x, V 0) +  f ( y, V 0) =  g( v 0 , V 0) = 0  .

(2)

With

X

³

X

´

h :=

f ( →

e)

=

g( e, v 0 , v 0)  ,

→

~

e ∈ E0 ( x,y)

e ∈E0( x,y)

(F2) for  f  requires

0 =  f ( x, V ) =  f (  →

e 0) +  h +  f ( x, V 0)

and

0 =  f ( y, V ) =  f (  ←

e 0)  − h +  f ( y, V 0)  ,

so we have to set

f (  →

e 0) :=  −f ( x, V 0)  − h  and  f (  ←

e 0) :=  −f ( y, V 0) +  h .

Then  f (  →

e 0) +  f (  ←

e 0) = 0 by (2), so  f  also satisfies (F1) in  e 0.

¤

flow

The polynomial  P  of Theorem 6.3.1 is known as the  flow polynomial polynomial

of  G.

[ 6.4.5 ]

Corollary 6.3.2.  If H and H0 are two finite abelian groups of equal order, then G has an H-flow if and only if G has an H0-flow.

¤

Corollary 6.3.2 has fundamental implications for the theory of al-

gebraic flows: it indicates that crucial difficulties in existence proofs of H-flows are unlikely to be of a group-theoretic nature. On the other hand, being able to choose a convenient group can be quite helpful; we

shall see a pretty example for this in Proposition 6.4.5.

6.3 Group-valued flows

131

Let  k > 1 be an integer and  G = ( V, E) a multigraph. A Z-flow  f k

→

on  G  such that 0  < |f ( →

e) | < k  for all  →

e ∈ E  is called a  k-flow . Clearly,

k-flow

any  k-flow is also an  `-flow for all  ` > k. Thus, we may ask which is the least integer  k  such that  G  admits a  k-flow—assuming that such a  k exists. We call this least  k  the  flow number  of  G  and denote it by  ϕ( G); flow

number

if  G  has no  k-flow for any  k, we put  ϕ( G) :=  ∞.

ϕ( G)

The task of determining flow numbers quickly leads to some of the

deepest open problems in graph theory. We shall consider these later

in the chapter. First, however, let us see how  k-flows are related to the more general concept of  H-flows.

There is an intimate connection between  k-flows and Z k-flows. Let σk  denote the natural homomorphism  i 7→ i  from Z to Z k. By compo-σk

sition with  σk, every  k-flow defines a Z k-flow. As the following theorem shows, the converse holds too: from every Z k-flow on  G  we can construct a  k-flow on  G. In view of Corollary 6.3.2, this means that the general question about the existence of  H-flows for arbitrary groups  H  reduces to the corresponding question for  k-flows.

Theorem 6.3.3. (Tutte 1950)

[ 6.4.1 ]

A multigraph admits a k-flow if and only if it admits a  Z

[ 6.4.2 ]

k -flow.

[ 6.4.3 ]

[ 6.4.5 ]

Proof . Let  g  be a Z k-flow on a multigraph  G = ( V, E); we construct a k-flow  f  on  G. We may assume without loss of generality that  G  has g

→

no loops. Let  F  be the set of all functions  f :  E →  Z that satisfy (F1), F

|

→

f ( →

e) | < k  for all  →

e ∈ E, and  σk ◦ f =  g; note that, like  g, any  f ∈ F  is nowhere zero.

Let us show first that  F 6=  ∅. Since we can express every value g( →

e)  ∈  Z k  as  i  with  |i| < k  and then put  f ( →

e) :=  i, there is clearly a map

→

→

f :  E →  Z such that  |f ( →

e) | < k  for all  →

e ∈ E  and  σk ◦ f =  g. For each edge

e ∈ E, let us choose one of its two directions and denote this by  →

e. We

→

may then define  f 0:  E →  Z by setting  f0( →

e) :=  f ( →

e) and  f 0( ←

e) :=  −f ( →

e)

for every  e ∈ E. Then  f 0  is a function satisfying (F1) and with values in the desired range; it remains to show that  σk ◦ f 0  and  g  agree not only on the chosen directions  →

e  but also on their inverses  ←

e. Since  σk  is a

homomorphism, this is indeed so:

( σk ◦ f 0)( ←

e) =  σk( −f ( →

e)) =  −( σk ◦ f )( →

e) =  −g( →

e) =  g( ←

e)  .

Hence  f 0 ∈ F , so  F  is indeed non-empty.

Our aim is to find an  f ∈ F  that satisfies Kirchhoff’s law (F2), and is thus a  k-flow. As a candidate, let us consider an  f ∈ F  for which the f

sum

132

6. Flows

X

K

K( f ) :=

|f( x, V ) |

x∈V

of all deviations from Kirchhoff’s law is least possible. We shall prove that  K( f ) = 0; then, clearly,  f ( x, V ) = 0 for every  x, as desired.

P

Suppose  K( f )  6= 0. Since  f  satisfies (F1), and hence f ( x, V ) =

x∈V

x

f ( V, V ) = 0, there exists a vertex  x  with

f ( x, V )  >  0  .

(1)

X

Let  X ⊆ V  be the set of all vertices  x0  for which  G  contains a walk x 0 e 0  . . . è− 1 x`  from  x  to  x0  such that  f( ei, xi, xi+1)  >  0 for all  i < `; X0

furthermore, let  X0 :=  X  r  { x }.

We first show that  X0  contains a vertex  x0  with  f ( x0, V )  <  0. By definition of  X, we have  f ( e, x0, y) 6 0 for all edges  e =  x0y  such that x0 ∈ X  and  y ∈ X. In particular, this holds for  x0 =  x. Thus, (1) implies f ( x, X0)  >  0. Then  f ( X0, x)  <  0 by (F1), as well as  f ( X0, X0) = 0.

Therefore

X  f( x0,V ) =  f( X0,V ) =  f( X0,X)+ f( X0,x)+ f( X0,X0)  <  0 , x0 ∈X0

x0

so some  x0 ∈ X0  must indeed satisfy

f ( x0, V )  <  0  .

(2)

W

As  x0 ∈ X, there is an  x– x0  walk  W =  x 0 e 0  . . . è− 1 x`  such that f ( ei, xi, xi+1)  >  0 for all  i < `. We now modify  f  by sending some flow

→

f 0

back along  W , letting  f 0:  E →  Z be given by





  f( →e)  − k  for  →e = ( ei, xi, xi+1),  i = 0 , . . . , ` −  1; f 0:  →

e 7→   f( →e)+ k  for  →e = ( ei,xi+1 ,xi),  i = 0 ,...,`− 1;

  f( →e)

for  e /

∈ W .

→

By definition of  W , we have  |f 0( →

e) | < k  for all  →

e ∈ E. Hence  f 0, like  f ,

lies in  F .

How does the modification of  f  affect  K? At all inner vertices  v of  W , as well as outside  W , the deviation from Kirchhoff’s law remains unchanged:

f 0( v, V ) =  f ( v, V )

for all  v ∈ V  r  { x, x0 }.

(3)

For  x  and  x0, on the other hand, we have

f 0( x, V ) =  f ( x, V )  − k  and  f 0( x0, V ) =  f ( x0, V ) +  k .

(4)

6.3 Group-valued flows

133

Since  g  is a Z k-flow and hence

σk( f ( x, V )) =  g( x, V ) = 0  ∈  Z k and

σk( f ( x0, V )) =  g( x0, V ) = 0  ∈  Z k , f ( x, V ) and  f ( x0, V ) are both multiples of  k. Thus  f ( x, V ) >  k  and f ( x0, V ) 6  −k, by (1) and (2). But then (4) implies that

|f0( x, V ) | < |f( x, V ) |  and  |f0( x0, V ) | < |f( x0, V ) | .

Together with (3), this gives  K( f 0)  < K( f ), a contradiction to the choice of  f .

Therefore  K( f ) = 0 as claimed, and  f  is indeed a  k-flow.

¤

Since the sum of two Z k-circulations is always another Z k-circulation, Z k-flows are often easier to construct (by summing over suitable partial flows) than  k-flows. In this way, Theorem 6.3.3 may be of considerable help in determining whether or not some given graph has a  k-flow. In the following sections we shall meet a number of examples for this.

6.4  k-Flows for small  k

Trivially, a graph has a 1-flow (the empty set) if and only if it has no edges. In this section we collect a few simple examples of sufficient

conditions under which a graph has a 2-, 3- or 4-flow. More examples

can be found in the exercises.

Proposition 6.4.1.  A graph has a 2-flow if and only if all its degrees

[ 6.6.1 ]

are even.

Proof . By Theorem 6.3.3, a graph  G = ( V, E) has a 2-flow if and only if

(6.3.3)

→

it has a Z2-flow, i.e. if and only if the constant map  E →  Z2 with value 1

satisfies (F2). This is the case if and only if all degrees are even.

¤

even

For the remainder of this chapter, let us call a graph  even  if all its vertex graph

degrees are even.

Proposition 6.4.2.  A cubic graph has a 3-flow if and only if it is bipartite.

134

6. Flows

(1.6.1)

Proof . Let  G = ( V, E) be a cubic graph. Let us assume first that

(6.3.3)

G  has a 3-flow, and hence also a Z3-flow  f . We show that any cycle C =  x 0  . . . x`x 0 in  G  has even length (cf. Proposition 1.6.1). Consider two consecutive edges on  C, say  ei− 1 :=  xi− 1 xi  and  ei :=  xixi+1. If  f assigned the same value to these edges in the direction of the forward

orientation of  C, i.e. if  f ( ei− 1 , xi− 1 , xi) =  f( ei, xi, xi+1), then  f  could not satisfy (F2) at  xi  for any non-zero value of the third edge at  xi.

Therefore  f  assigns the values 1 and 2 to the edges of  C  alternately, and in particular  C  has even length.

Conversely, let  G  be bipartite, with vertex bipartition  { X, Y }.

→

Since  G  is cubic, the map  E →  Z3 defined by  f ( e, x, y) := 1 and f ( e, y, x) := 2 for all edges  e =  xy  with  x ∈ X  and  y ∈ Y  is a Z3-flow on  G. By Theorem 6.3.3, then,  G  has a 3-flow.

¤

What are the flow numbers of the complete graphs  Kn? For odd

n >  1, we have  ϕ( Kn) = 2 by Proposition 6.4.1. Moreover,  ϕ( K 2) =  ∞, and  ϕ( K 4) = 4; this is easy to see directly (and it follows from Propositions 6.4.2 and 6.4.5). Interestingly,  K 4 is the only complete graph with flow number 4:

Proposition 6.4.3.  For all even n >  4 , ϕ( Kn) = 3 .

(6.3.3)

Proof . Proposition 6.4.1 implies that  ϕ( Kn) > 3 for even  n. We show, by induction on  n, that every  G =  Kn  with even  n >  4 has a 3-flow.

For the induction start, let  n = 6. Then  G  is the edge-disjoint union of three graphs  G 1,  G 2,  G 3, with  G 1 , G 2 =  K 3 and  G 3 =  K 3 ,  3. Clearly G 1 and  G 2 each have a 2-flow, while  G 3 has a 3-flow by Proposition 6.4.2.

The union of all these flows is a 3-flow on  G.

Now let  n >  6, and assume the assertion holds for  n −  2. Clearly,  G  is the edge-disjoint union of a  Kn− 2 and a graph  G0 = ( V 0, E0) with  G0 =

Kn− 2  ∗ K 2. The  Kn− 2 has a 3-flow by induction. By Theorem 6.3.3, it thus suffices to find a Z3-flow on  G0. For every vertex  z  of the  Kn− 2  ⊆ G0, let  fz  be a Z3-flow on the triangle  zxyz ⊆ G0, where  e =  xy  is the edge

→

of the  K 2 in  G0. Let  f :  E0 →  Z3 be the sum of these flows. Clearly,  f  is nowhere zero, except possibly in ( e, x, y) and ( e, y, x). If  f ( e, x, y)  6= 0, then  f  is the desired Z3-flow on  G0. If  f ( e, x, y) = 0, then  f +  fz (for any  z) is a Z3-flow on  G0.

¤

Proposition 6.4.4.  Every 4-edge-connected graph has a 4-flow.

(3.5.2)

Proof . Let  G  be a 4-edge-connected graph. By Corollary 3.5.2,  G  has two edge-disjoint spanning trees  Ti,  i = 1 ,  2. For each edge  e /

∈ Ti, let

f 1 ,e, f 2 ,e

Ci,e  be the unique cycle in  Ti +  e, and let  fi,e  be a Z4-flow of value  i around  Ci,e—more precisely: a Z4-circulation on  G  with values  i  and  −i on the edges of  Ci,e  and zero otherwise.

6.4  k-Flows for small  k

135

P

Let  f 1 :=

f

e /

∈T

1 ,e. Since each  e /

∈ T 1 lies on only one cycle  C 1 ,e0

f 1

1

(namely, for  e =  e0),  f 1 takes only the values 1 and  − 1 (= 3) outside  T 1.

Let

F :=  { e ∈ E( T 1)  | f 1( e) = 0  }

P

and  f 2 :=

f

e∈F

2 ,e.

As above,  f 2( e) = 2 =  − 2 for all  e ∈ F . Now f 2

f :=  f 1 +  f 2 is the sum of Z4-circulations, and hence itself a Z4-circula-f

tion. Moreover,  f  is nowhere zero: on edges in  F  it takes the value 2, on edges of  T 1  − F  it agrees with  f 1 (and is hence non-zero by the choice of  F ), and on all edges outside  T 1 it takes one of the values 1 or 3. Hence, f  is a Z4-flow on  G, and the assertion follows by Theorem 6.3.3.

¤

The following proposition describes the graphs with a 4-flow in terms

of those with a 2-flow:

Proposition 6.4.5.

(i)  A graph has a 4-flow if and only if it is the union of two even subgraphs.

(ii)  A cubic graph has a 4-flow if and only if it is 3-edge-colourable.

Proof . Let Z22 = Z2  ×  Z2 be the Klein four-group. (Thus, the elements of

(6.3.2)

(6.3.3)

Z22 are the pairs ( a, b) with  a, b ∈  Z2, and ( a, b)+( a0, b0) = ( a+ a0, b+ b0).) By Corollary 6.3.2 and Theorem 6.3.3, a graph has a 4-flow if and only if it has a Z22 -flow.

(i) now follows directly from Proposition 6.4.1.

(ii) Let  G = ( V, E) be a cubic graph. We assume first that  G  has a Z2

r  {

2 -flow  f , and define an edge colouring  E →  Z2

2

0  }. As  a =  −a  for

→

all  a ∈  Z22, we have  f( →

e) =  f ( ←

e) for every  →

e ∈ E; let us colour the edge

e  with this colour  f ( →

e). Now if two edges with a common end  v  had

the same colour, then these two values of  f  would sum to zero; by (F2), f  would then assign zero to the third edge at  v. As this contradicts the definition of  f , our edge colouring is correct.

Conversely, since the three non-zero elements of Z22 sum to zero,

every 3-edge-colouring  c:  E →  Z2 r  {

2

0  }  defines a Z22 -flow on  G  by letting

→

f ( →

e) =  f ( ←

e) =  c( e) for all  →

e ∈ E.

¤

Corollary 6.4.6.  Every cubic 3-edge-colourable graph is bridgeless.

¤

136

6. Flows

6.5 Flow-colouring duality

In this section we shall see a surprising connection between flows and

colouring: every  k-flow on a plane multigraph gives rise to a  k-vertex-colouring of its dual, and vice versa. In this way, the investigation of k-flows appears as a natural generalization of the familiar map colouring problems in the plane.

G = ( V, E)

Let  G = ( V, E) and  G∗ = ( V ∗, E∗) be dual plane multigraphs. For G∗

simplicity, let us assume that  G  and  G∗  have neither bridges nor loops and are non-trivial. For edge sets  F ⊆ E, let us write

F ∗

F ∗ :=  { e∗ ∈ E∗ | e ∈ F } .

Conversely, if a subset of  E∗  is given, we shall usually write it immediately in the form  F ∗, and thus let  F ⊆ E  be defined implicitly via the bijection  e 7→ e∗.

Suppose we are given a circulation  g  on  G∗: how can we employ the duality between  G  and  G∗  to derive from  g  some information about  G?

The most general property of all circulations is Proposition 6.1.1, which says that  g( X, X) = 0 for all  X ⊆ V ∗. By Proposition 4.6.1, the minimal cuts  E∗( X, X) in  G∗  correspond precisely to the cycles in  G. Thus if we take the composition  f  of the maps  e 7→ e∗  and  g, and sum its values over the edges of a cycle in  G, then this sum should again be zero.

Of course, there is still a technical hitch: since  g  takes its arguments

→

not in  E∗  but in  E∗, we cannot simply define  f  as above: we first have

→

→

to refine the bijection  e 7→ e∗  into one from  E  to  E∗, i.e. assign to every

→

→

e ∈ E  canonically one of the two directions of  e∗. This will be the purpose of our first lemma. After that, we shall show that  f  does indeed sum to zero along any cycle in  G.

If  C =  v 0  . . . v`− 1 v 0 is a cycle with edges  ei =  vivi+1 (and  v` :=  v 0), we shall call

→

→

C

C :=  { ( ei, vi, vi+1)  | i < ` }

→

cycle with

a  cycle with orientation. Note that this definition of  C  depends on the orientation

vertex enumeration chosen to denote  C: every cycle has two orientations.

→

Conversely, of course,  C  can be reconstructed from the set  C . In practice,

→

we shall therefore speak about  C  freely even when, formally, only  C  has been defined.

→

→

Lemma 6.5.1.  There exists a bijection ∗:  →

e 7→ →

e ∗ from E to E∗ with

the following properties.

(i)  The underlying edge of →

e ∗ is always e∗, i.e. →

e ∗ is one of the two

directions →

e∗, ←

e∗ of e∗.

(ii)  If C ⊆ G is a cycle, F :=  E( C) , and if X ⊆ V ∗ is such that

→

F ∗ =  E∗( X, X) , then there exists an orientation C of C with

{ →

→

→

e ∗ | →

e ∈ C } =  E∗( X, X) .

6.5 Flow-colouring duality

137

The proof of Lemma 6.5.1 is not entirely trivial: it is based on the

so-called  orientability  of the plane, and we cannot give it here. Still, the assertion of the lemma is intuitively plausible. Indeed if we define for  e =  vw  and  e∗ =  xy  the assignment ( e, v, w)  7→ ( e, v, w) ∗ ∈

{ ( e∗, x, y) , ( e∗, y, x)  }  simply by turning  e  and its ends clockwise onto  e∗

(Fig. 6.5.1), then the resulting map  →

e 7→ →

e ∗  satisfies the two assertions

of the lemma.

X

→

C

X

Fig. 6.5.1.  Oriented cycle-cut duality

→

→

Given an abelian group  H, let  f :  E → H  and  g:  E∗ → H  be two maps f, g

such that

f ( →

e) =  g( →

e ∗)

→

→

for all  →

e ∈ E. For  F ⊆ →

E, we set

X

→

f ( F ) :=

f ( →

e)  .

→

~

e ∈ ~

F

f ( C ) etc .

Lemma 6.5.2.

(i)  The map g satisfies (F1) if and only if f does.

(ii)  The map g is a circulation on G∗ if and only if f satisfies (F1)

→

→

and f ( C ) = 0  for every cycle C with orientation.

Proof . Assertion (i) follows from Lemma 6.5.1 (i) and the fact that

(4.6.1)

(6.1.1)

→

e 7→ →

e ∗  is bijective.

For the forward implication of (ii), let us assume that  g  is a circulation on  G∗, and consider a cycle  C ⊆ G  with some given orientation.

Let  F :=  E( C). By Proposition 4.6.1,  F ∗  is a minimal cut in  G∗, i.e.

F ∗ =  E∗( X, X) for some suitable  X ⊆ V ∗. By definition of  f  and  g,

Lemma 6.5.1 (ii) and Proposition 6.1.1 give X

X

→

→

f ( C ) =

f ( →

e) =

g( d) =  g( X, X) = 0

→

~

e ∈ ~

C

~

d ∈E∗( X,X)

→

←

→

for one of the two orientations  C  of  C. Then, by  f ( C ) =  −f ( C ), also

138

6. Flows

the corresponding value for our given orientation of  C  must be zero.

For the backward implication it suffices by (i) to show that  g  satisfies (F2), i.e. that  g( x, V ∗) = 0 for every  x ∈ V ∗. We shall prove that g( x, V ( B)) = 0 for every block  B  of  G∗  containing  x; since every edge of  G∗  at  x  lies in exactly one such block, this will imply  g( x, V ∗) = 0.

B

So let  x ∈ V ∗  be given, and let  B  be any block of  G∗  containing  x. Since  G∗  is a non-trivial plane dual, and hence connected, we F ∗, F

have  B − x 6=  ∅. Let  F ∗  be the set of all edges of  B  at  x (Fig. 6.5.2),

X

X

C

x

F ∗

B

Fig. 6.5.2.  The cut  F ∗  in  G∗

X

and let  X  be the vertex set of the component of  G∗ − F ∗  containing  x.

Then  ∅ 6=  V ( B − x)  ⊆ X, by the maximality of  B  as a cutvertex-free subgraph. Hence

F ∗ =  E∗( X, X)

(1)

by definition of  X, i.e.  F ∗  is a cut in  G∗. As a dual,  G∗  is connected, so  G∗[  X ] too is connected. Indeed, every vertex of  X  is linked to  x  by a path  P ⊆ G∗  whose last edge lies in  F ∗. Then  P − x  is a path in G∗[  X ] meeting  B. Since  x  does not separate  B, this shows that  G∗[  X ]

is connected.

Thus,  X  and  X  are both connected in  G∗, so  F ∗  is even a minimal C

cut in  G∗. Let  C ⊆ G  be the cycle with  E( C) =  F  that exists by

→

Proposition 4.6.1. By Lemma 6.5.1 (ii),  C  has an orientation  C  such that

{ →

→

→

→

→

e ∗ | →

e ∈ C } =  E∗( X, X). By (1), however,  E∗( X, X) =  E∗( x, V ( B)), so

→

g( x, V ( B)) =  g( X, X) =  f ( C ) = 0

by definition of  f  and  g.

¤

With the help of Lemma 6.5.2, we can now prove our colouring-flow

duality theorem for plane multigraphs. If  P =  v 0  . . . v`  is a path with edges  ei =  vivi+1 ( i < `), we set (depending on our vertex enumeration of  P )

→

→

P

P :=  { ( ei, vi, vi+1)  | i < ` }

v

→

→

0  → v`

and call  P  a  v

path

0  → v` path . Again,  P  may be given implicitly by  P .

6.5 Flow-colouring duality

139

Theorem 6.5.3. (Tutte 1954)

For every dual pair G, G∗ of plane multigraphs,

χ( G) =  ϕ( G∗)  .

Proof . Let  G =: ( V, E) and  G∗ =: ( V ∗, E∗). For  |G| ∈ {  1 ,  2  }  the

(1.5.5)

assertion is easily checked; we shall assume that  |G| > 3, and apply V, E

induction on the number of bridges in  G. If  e ∈ G  is a bridge then  e∗

V ∗, E∗

is a loop, and  G∗ − e∗  is a plane dual of  G/e (why?). Hence, by the induction hypothesis,

χ( G) =  χ( G/e) =  ϕ( G∗ − e∗) =  ϕ( G∗) ; for the first and the last equality we use that, by  |G| > 3,  e  is not the only edge of  G.

So all that remains to be checked is the induction start: let us

assume that  G  has no bridge. If  G  has a loop, then  G∗  has a bridge, and  χ( G) =  ∞ =  ϕ( G∗) by convention. So we may also assume that  G

has no loop. Then  χ( G) is finite; we shall prove for given  k > 2 that  G

k

is  k-colourable if and only if  G∗  has a  k-flow. As  G—and hence  G∗—

has neither loops nor bridges, we may apply Lemmas 6.5.1 and 6.5.2

→

→

to  G  and  G∗. Let  →

e 7→ →

e ∗  be the bijection between  E  and  E∗  from

Lemma 6.5.1.

We first assume that  G∗  has a  k-flow. Then  G∗  also has a Z k-flow  g.

g

→

As before, let  f :  E →  Z k  be defined by  f ( →

e) :=  g( →

e ∗). We shall use  f  to

f

define a vertex colouring  c:  V →  Z k  of  G.

Let  T  be a normal spanning tree of  G, with root  r, say. Put  c( r) := 0.

→

→

For every other vertex  v ∈ V  let  c( v) :=  f ( P ), where  P  is the  r → v path in  T . To check that this is a proper colouring, consider an edge e =  vw ∈ E. As  T  is normal, we may assume that  v < w  in the tree order of  T . If  e  is an edge of  T  then  c( w)  − c( v) =  f ( e, v, w) by definition of  c, so  c( v)  6=  c( w) since  g (and hence  f ) is nowhere zero. If  e /

∈ T , let

→

P  denote the  v → w  path in  T . Then

→

c( w)  − c( v) =  f ( P ) =  −f ( e, w, v)  6= 0

by Lemma 6.5.2 (ii).

Conversely, we now assume that  G  has a  k-colouring  c. Let us define c

→

f :  E →  Z by

f ( e, v, w) :=  c( w)  − c( v)  , f

→

and  g:  E∗ →  Z by  g( →

e ∗) :=  f ( →

e). Clearly,  f  satisfies (F1) and takes

g

values in  { ± 1 , . . . , ±( k −  1)  }, so by Lemma 6.5.2 (i) the same holds

→

→

for  g. By definition of  f , we further have  f ( C ) = 0 for every cycle  C

with orientation. By Lemma 6.5.2 (ii), therefore,  g  is a  k-flow.

¤

140

6. Flows

6.6 Tutte’s flow conjectures

How can we determine the flow number of a graph? Indeed, does every

(bridgeless) graph have a flow number, a  k-flow for some  k? Can flow numbers, like chromatic numbers, become arbitrarily large? Can we

characterize the graphs admitting a  k-flow, for given  k?

Of these four questions, we shall answer the second and third in this

section: we prove that every bridgeless graph has a 6-flow. In particular, a graph has a flow number if and only if it has no bridge. The question asking for a characterization of the graphs with a  k-flow remains interesting for  k = 3 ,  4 ,  5. Partial answers are suggested by the following three conjectures of Tutte, who initiated algebraic flow theory.

The oldest and best known of the Tutte conjectures is his  5-flow

conjecture:

Five-Flow Conjecture. (Tutte 1954)

Every bridgeless multigraph has a 5-flow.

Which graphs have a 4-flow? By Proposition 6.4.4, the 4-edge-connected graphs are among them. The Petersen graph (Fig. 6.6.1), on

the other hand, is an example of a bridgeless graph without a 4-flow:

since it is cubic but not 3-edge-colourable (Ex. 19, Ch. 5), it cannot have a 4-flow by Proposition 6.4.5 (ii).

Fig. 6.6.1.  The Petersen graph

Tutte’s  4-flow conjecture  states that the Petersen graph must be present in every graph without a 4-flow:

Four-Flow Conjecture. (Tutte 1966)

Every bridgeless multigraph not containing the Petersen graph as a minor has a 4-flow.

By Proposition 1.7.2, we may replace the word ‘minor’ in the 4-flow conjecture by ‘topological minor’.

6.6 Tutte’s flow conjectures

141

Even if true, the 4-flow conjecture will not be best possible: a  K 11, for example, contains the Petersen graph as a minor but has a 4-flow,

even a 2-flow. The conjecture appears more natural for sparser graphs,

and indeed the cubic graphs form an important special case. (See the

notes.)

A cubic bridgeless graph or multigraph without a 4-flow (equivalent-

ly, without a 3-edge-colouring) is called a  snark . The 4-flow conjecture snark

for cubic graphs says that every snark contains the Petersen graph as

a minor; in this sense, the Petersen graph has thus been shown to be

the smallest snark. Snarks form the hard core both of the four colour

theorem and of the 5-flow conjecture: the four colour theorem is equivalent to the assertion that no snark is planar (exercise), and it is not difficult to reduce the 5-flow conjecture to the case of snarks.5 However, although the snarks form a very special class of graphs, none of the

problems mentioned seems to become much easier by this reduction.6

Three-Flow Conjecture. (Tutte 1972)

Every multigraph without a cut consisting of exactly one or exactly three edges has a 3-flow.

Again, the 3-flow conjecture will not be best possible: it is easy to construct graphs with three-edge cuts that have a 3-flow (exercise).

By our duality theorem (6.5.3), all three flow conjectures are true for planar graphs and thus motivated: the 3-flow conjecture translates

to Gr¨

otzsch’s theorem (5.1.3), the 4-flow conjecture to the four colour

theorem (since the Petersen graph is not planar, it is not a minor of a planar graph), the 5-flow conjecture to the five colour theorem.

We finish this section with the main result of the chapter:

Theorem 6.6.1. (Seymour 1981)

Every bridgeless graph has a 6-flow.

(3.3.5)

Proof . Let  G = ( V, E) be a bridgeless graph. Since 6-flows on the

(6.1.1)

(6.4.1)

components of  G  will add up to a 6-flow on  G, we may assume that G  is connected; as  G  is bridgeless, it is then 2-edge-connected. Note that any two vertices in a 2-edge-connected graph lie in some common

even connected subgraph—for example, in the union of two edge-disjoint

paths linking these vertices by Menger’s theorem (3.3.5 (ii)). We shall use this fact repeatedly.

5 The same applies to another well-known conjecture, the  cycle double cover con-

jecture; see Exercise 13.

6 That snarks are elusive has been known to mathematicians for some time; cf.

Lewis Carroll,  The Hunting of the Snark , Macmillan 1876.

142

6. Flows

H 0 , . . . , Hn

We shall construct a sequence  H 0 , . . . , Hn  of disjoint connected and F 1 , . . . , Fn

even subgraphs of  G, together with a sequence  F 1 , . . . , Fn  of non-empty sets of edges between them. The sets  Fi  will each contain only one or Vi, Ei

two edges, between  Hi  and  H 0  ∪ . . . ∪ Hi− 1. We write  Hi =: ( Vi, Ei), Hi

Hi := ( H 0  ∪ . . . ∪ Hi) + ( F 1  ∪ . . . ∪ Fi) V i, Ei

and  Hi =: ( V i, Ei). Note that each  Hi = ( Hi− 1  ∪ Hi) +  Fi  is connected (induction on  i). Our assumption that  Hi  is even implies by Proposition

6.4.1 (or directly by Proposition 1.2.1) that  Hi  has no bridge.

As  H 0 we choose any  K 1 in  G. Now assume that  H 0 , . . . , Hi− 1 and F 1 , . . . , Fi− 1 have been defined for some  i >  0. If  V i− 1 =  V , we terminate n

the construction and set  i −  1 =:  n. Otherwise, we let  Xi ⊆ V i− 1 be Xi

minimal such that  Xi 6=  ∅  and

¯¯

¯

E( X

¯

i, V i− 1 r  Xi) 6 1

(1)

(Fig. 6.6.2); such an  Xi  exists, because  V i− 1 is a candidate. Since  G

is 2-edge-connected, (1) implies that  E( Xi, V i− 1)  6=  ∅. By the minimality of  Xi, the graph  G [  Xi ] is connected and bridgeless, i.e. 2-edge-Fi

connected or a  K 1. As the elements of  Fi  we pick one or two edges from  E( Xi, V i− 1), if possible two. As  Hi  we choose any connected even subgraph of  G [  Xi ] containing the ends in  Xi  of the edges in  Fi.

V i− 1 r  Xi

Hi− 1

Hi

Fi

Xi

V i− 1

Fig. 6.6.2.  Constructing the  Hi  and  Fi

H

When our construction is complete, we set  Hn =:  H  and  E0 :=

E0

E  r  E( H). By definition of  n,  H  is a spanning connected subgraph of  G.

fn, . . . , f 0

We now define, by ‘reverse’ induction, a sequence  fn, . . . , f 0 of Z3-

→

→

Ce

circulations on  G. For every edge  e ∈ E0, let  Ce  be a cycle (with orienta-

→

tion) in  H +  e  containing  e, and  fe  a positive flow around  Ce; formally,

→

→

fe

we let  fe  be a Z3-circulation on  G  such that  f − 1

e

(0) =  E  r ( Ce ∪ ←

Ce).

fn

Let  fn  be the sum of all these  fe. Since each  e0 ∈ E0  lies on just one of

→

the cycles  Ce (namely, on  Ce0), we have  fn( →

e)  6= 0 for all  →

e ∈ E0.

6.6 Tutte’s flow conjectures

143

Assume now that Z3-circulations  fn, . . . , fi  on  G  have been defined fi

for some  i  6  n, and that

→

[  →

fi( →

e)  6= 0 for all  →

e ∈ E0 ∪

Fj ,

(2)

j>i

→

→

where  Fj :=  { →

e ∈ E | e ∈ Fj }. Our aim is to define  fi− 1 in such a way

→

Fj

that (2) also holds for  i −  1.

We first consider the case that  |Fi| = 1, say  Fi =  { e }. We then e

let  fi− 1 :=  fi, and thus have to show that  fi  is non-zero on (the two directions of)  e. Our assumption of  |Fi| = 1 implies by the choice of Fi  that  G  contains no  Xi– V i− 1 edge other than  e. Since  G  is 2-edge-connected, it therefore has at least—and thus, by (1), exactly—one edge e0  between  Xi  and  V i− 1 r  Xi. We show that  fi  is non-zero on  e0; as e0

{ e, e0 }  is a cut in  G, this implies by Proposition 6.1.1 that  fi  is also non-zero on  e.

To show that  fi  is non-zero on  e0, we use (2): we show that  e0 ∈

S

E0 ∪

F

j>i

j , i.e. that  e0  lies in no  Hk  and in no  Fj  with  j  6  i. Since  e0

has both ends in  V i− 1, it clearly lies in no  Fj  with  j  6  i  and in no  Hk with  k < i. But every  Hk  with  k >  i  is a subgraph of  G [  V i− 1 ]. Since  e0

is a bridge of  G [  V i− 1 ] but  Hk  has no bridge, this means that  e0 /

∈ Hk.

Hence,  fi− 1 does indeed satisfy (2) for  i −  1 in the case considered.

It remains to consider the case that  |Fi| = 2, say  Fi =  { e 1 , e 2  }.

e 1 , e 2

Since  Hi  and  Hi− 1 are both connected, we can find a cycle  C  in  Hi =

C

( Hi ∪ Hi− 1) +  Fi  that contains  e 1 and  e 2. If  fi  is non-zero on both these edges, we again let  fi− 1 :=  fi. Otherwise, there are directions  →

e 1 and

→

e 2 of  e 1 and  e 2 such that, without loss of generality,  fi(  →

e 1) = 0 and

→

→

fi(  →

e 2)  ∈ {  0 ,  1  }. Let  C  be the orientation of  C  with  →

e 2  ∈ C , and let  g  be

→

a flow of value 1 around  C (formally: let  g  be a Z3-circulation on  G  such

→

→

that  g(  →

e 2) = 1 and  g− 1(0) =  E  r ( C ∪ ←

C )). We then let  fi− 1 :=  fi +  g.

By choice of the directions  →

e 1 and  →

e 2,  fi− 1 is non-zero on both edges.

→

S

→

Since  fi− 1 agrees with  fi  on all of  E0 ∪

F

j>i

j  and (2) holds for  i, we

again have (2) also for  i −  1.

Eventually,  f 0 will be a Z3-circulation on  G  that is nowhere zero except possibly on edges of  H 0  ∪ . . . ∪ Hn. Composing  f 0 with the map h 7→  2 h  from Z3 to Z6 ( h ∈ {  1 ,  2  }), we obtain a Z6-circulation  f  on  G

f

with values in  {  0 ,  2 ,  4  }  for all edges lying in some  Hi, and with values in  {  2 ,  4  }  for all other edges. Adding to  f  a 2-flow on each  Hi (formally: a Z6-circulation on  G  with values in  {  1 , − 1  }  on the edges of  Hi  and 0

otherwise; this exists by Proposition 6.4.1), we obtain a Z6-circulation on  G  that is nowhere zero. Hence,  G  has a 6-flow by Theorem 6.3.3.

¤

144

6. Flows

Exercises

1.  −  Prove Proposition 6.2.1 by induction on  |S|.

2.

(i) −  Given  n ∈  N, find a capacity function for the network below such that the algorithm from the proof of the max-flow min-cut theorem will need more than  n  augmenting paths  W  if these are badly chosen.

s

t

(ii)+ Show that, if all augmenting paths are chosen as short as possible, their number is bounded by a function of the size of the network.

3.+ Derive Menger’s Theorem 3.3.4 from the max-flow min-cut theorem.

(Hint. The edge version is easy. For the vertex version, apply the edge version to a suitable auxiliary graph.)

4.  −  Let  f  be an  H-circulation on  G  and  g:  H → H0  a group homomorphism.

Show that  g ◦ f  is an  H0-circulation on  G. Is  g ◦ f  an  H0-flow if  f  is an H-flow?

5.  −  Given  k > 1, show that a graph has a  k-flow if and only if each of its blocks has a  k-flow.

6.  −  Show that  ϕ( G/e) 6  ϕ( G) whenever  G  is a multigraph and  e  an edge of  G. Does this imply that, for every  k, the class of all multigraphs admitting a  k-flow is closed under taking minors?

7.  −  Work out the flow number of  K 4 directly, without using any results from the text.

8.

Let  H  be a finite abelian group,  G  a graph, and  T  a spanning tree of  G. Show that every mapping from the directions of  E( G) r  E( T ) to H  that satisfies (F1) extends uniquely to an  H-circulation on  G.

Do not use the 6-flow Theorem 6.6.1 for the following three exercises.

9.

Show that  ϕ( G)  < ∞  for every bridgeless multigraph  G.

10.

Assume that a graph  G  has  m  spanning trees such that no edge of  G

lies in all of these trees. Show that  ϕ( G) 6 2 m.

11.+ Let  G  be a bridgeless connected graph with  n  vertices and  m  edges. By considering a normal spanning tree of  G, show that  ϕ( G) 6  m − n + 2.

12.

Show that every graph with a Hamilton cycle has a 4-flow. (A  Hamilton cycle  of  G  is a cycle in  G  that contains all the vertices of  G.) 13.

A family of (not necessarily distinct) cycles in a graph  G  is called a cycle double cover  of  G  if every edge of  G  lies on exactly two of these cycles. The  cycle double cover conjecture  asserts that every bridgeless multigraph has a cycle double cover. Prove the conjecture for graphs

with a 4-flow.

Exercises

145

14.  −  Determine the flow number of  C 5  ∗ K 1, the wheel with 5 spokes.

15.

Find bridgeless graphs  G  and  H =  G − e  such that 2  < ϕ( G)  < ϕ( H).

16.

Prove Proposition 6.4.1 without using Theorem 6.3.3.

17.+ Prove  Heawood’s theorem  that a plane triangulation is 3-colourable if and only if all its vertices have even degree.

18.  −  Find a bridgeless graph that has both a 3-flow and a cut of exactly three edges.

19.

Show that the 3-flow conjecture for planar multigraphs is equivalent to

Gr¨

otzsch’s Theorem 5.1.3.

20.

(i) −  Show that the four colour theorem is equivalent to the non-existence of a planar snark, i.e. to the statement that every cubic bridgeless planar multigraph has a 4-flow.

(ii) Can ‘bridgeless’ in (i) be replaced by ‘3-connected’ ?

21.+ Show that a graph  G = ( V, E) has a  k-flow if and only if it admits an orientation  D  that directs, for every  X ⊆ V , at least 1 /k  of the edges in  E( X, X) from  X  towards  X.

22.  −  Generalize the 6-flow Theorem 6.6.1 to multigraphs.

Notes

Network flow theory is an application of graph theory that has had a major and lasting impact on its development over decades. As is illustrated already by the fact that Menger’s theorem can be deduced easily from the max-flow

min-cut theorem (Exercise 3), the interaction between graphs and networks may go either way: while ‘pure’ results in areas such as connectivity, matching and random graphs have found applications in network flows, the intuitive power of the latter has boosted the development of proof techniques that have in turn brought about theoretic advances.

The standard reference for network flows is L.R. Ford & D.R. Fulkerson, Flows in Networks, Princeton University Press 1962. A more recent and comprehensive account is given by R.K. Ahuja, T.L. Magnanti & J.B. Orlin,  Network flows, Prentice-Hall 1993. For more theoretical aspects, see A. Frank’s chapter in the  Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995. A general introduction to graph algorithms is given in A. Gibbons,  Algorithmic Graph Theory, Cambridge University Press 1985.

If one recasts the maximum flow problem in linear programming terms,

one can derive the max-flow min-cut theorem from the linear programming duality theorem; see A. Schrijver,  Theory of integer and linear programming, Wiley 1986.

The more algebraic theory of group-valued flows and  k-flows has been developed largely by Tutte; he gives a thorough account in his monograph W.T. Tutte,  Graph Theory, Addison-Wesley 1984. Tutte’s flow conjectures are

146

6. Flows

covered also in F. Jaeger’s survey, Nowhere-zero7 flow problems, in (L.W. Beineke & R.J. Wilson, eds.)  Selected Topics in Graph Theory 3, Academic Press 1988. For the flow conjectures, see also T.R. Jensen & B. Toft,  Graph Coloring Problems, Wiley 1995. Seymour’s 6-flow theorem is proved in P.D. Seymour, Nowhere-zero 6-flows,  J. Combin. Theory B 30 (1981), 130–135. This paper also indicates how Tutte’s 5-flow conjecture reduces to snarks. In 1998, Robertson, Sanders, Seymour and Thomas announced a proof of the 4-flow

conjecture for cubic graphs.

Finally, Tutte discovered a 2-variable polynomial associated with a graph, which generalizes both its chromatic polynomial and its flow polynomial.

What little is known about this  Tutte polynomial  can hardly be more than the tip of the iceberg: it has far-reaching, and largely unexplored, connections to areas as diverse as knot theory and statistical physics. See D.J.A. Welsh, Complexity: knots, colourings and counting (LMS Lecture Notes 186), Cambridge University Press 1993.

7 In the literature, the term ‘flow’ is often used to mean what we have called ‘circulation’, i.e. flows are not required to be nowhere zero unless this is stated explicitly.

7

Substructures in

Dense Graphs

In this chapter and the next, we study how global parameters of a graph, such as its edge density or chromatic number, have a bearing on the

existence of certain local substructures. How many edges, for instance, do we have to give a graph on  n  vertices to be sure that, no matter how these edges happen to be arranged, the graph will contain a  Kr  subgraph for some given  r? Or at least a  Kr  minor? Or a topological  Kr  minor?

Will some sufficiently high average degree or chromatic number ensure

that one of these substructures occurs?

Questions of this type are among the most natural ones in graph

theory, and there is a host of deep and interesting results. Collectively, these are known as  extremal graph theory.

Extremal graph problems in this sense fall neatly into two categories,

as follows. If we are looking for ways to ensure by global assumptions

that a graph  G  contains some given graph  H  as a  minor (or topological minor), it will suffice to raise  kGk  above the value of some linear function of  |G| (depending on  H), i.e. to make  ε( G) large enough. The existence of such a function was already established in Theorem 3.6.1. The precise growth rate needed will be investigated in Chapter 8, where we study

substructures of such ‘sparse’ graphs. Since a large enough value of  ε

gives rise to an  H  minor for any given graph  H, its occurrence could be forced alternatively by raising some other global invariants (such as  κ

or  χ) which, in turn, force up the value of  ε, at least in some subgraph.

This, too, will be a topic for Chapter 8.

On the other hand, if we ask what global assumptions might imply

the existence of some given graph  H  as a  subgraph, it will not help to raise any of the invariants  ε,  κ  or  χ, let alone any of the other invariants discussed in Chapter 1. Indeed, as mentioned in Chapter 5.2,

148

7. Substructures in Dense Graphs

given any graph  H  that contains at least one cycle, there are graphs of arbitrarily large chromatic number not containing  H  as a subgraph

(Theorem 11.2.2). By Corollary 5.2.3 and Theorem 1.4.2, such graphs have subgraphs of arbitrarily large average degree and connectivity, so these invariants too can be large without the presence of an  H  subgraph.

Thus, unless  H  is a forest, the only way to force the presence of an  H

subgraph in an arbitrary graph  G  by global assumptions on  G  is to raise kGk  substantially above any value implied by large values of the above invariants. If  H  is not bipartite, then any function  f  such that  f ( n) edges on  n  vertices force an  H  subgraph must even grow quadratically with  n: since complete bipartite graphs can have 1  n 2 edges,  f ( n) must 4

exceed 1  n 2.

4

dense

Graphs with a number of edges roughly1 quadratic in their number

±¡ ¢

of vertices are usually called  dense; the number  kGk |G| —the propor-2

edge

tion of its potential edges that  G  actually has—is the  edge density  of  G.

density

The question of exactly which edge density is needed to force a given

subgraph is the archetypal extremal graph problem in its original (nar-

rower) sense; it is the topic of this chapter. Rather than attempting to survey the wide field of (dense) extremal graph theory, however, we shall concentrate on its two most important results and portray one powerful

general proof technique.

The two results are Turán’s classic extremal graph theorem for

H =  Kr, a result that has served as a model for countless similar theorems for other graphs  H, and the fundamental Erd˝

os-Stone theo-

rem, which gives precise asymptotic information for all  H  at once (Section 7.1). The proof technique, one of increasing importance in the

extremal theory of dense graphs, is the use of the Szemerédi  regularity

lemma. This lemma is presented and proved in Section 7.2. In Section 7.3, we outline a general method for applying the regularity lemma, and illustrate this in the proof of the Erd˝

os-Stone theorem postponed

from Section 7.1. Another application of the regularity lemma will be given in Chapter 9.2.

7.1 Subgraphs

Let  H  be a graph and  n >  |H|. How many edges will suffice to force an H  subgraph in any graph on  n  vertices, no matter how these edges are arranged? Or, to rephrase the problem: which is the greatest possible

number of edges that a graph on  n  vertices can have  without  containing a copy of  H  as a subgraph? What will such a graph look like? Will it be unique?

1 Note that, formally, the notions of sparse and dense make sense only for families of graphs whose order tends to infinity, not for individual graphs.

7.1 Subgraphs

149

A graph  G 6⊇ H  on  n  vertices with the largest possible number of edges is called  extremal  for  n  and  H; its number of edges is denoted by extremal

ex( n, H). Clearly, any graph  G  that is extremal for some  n  and  H  will ex( n, H)

also be edge-maximal with  H 6⊆ G. Conversely, though, edge-maximality does not imply extremality:  G  may well be edge-maximal with  H 6⊆ G

while having fewer than ex( n, H) edges (Fig. 7.1.1).

Fig. 7.1.1.  Two graphs that are edge-maximal with  P  3  6⊆ G; is the right one extremal?

As a case in point, we consider our problem for  H =  Kr (with  r >  1).

A moment’s thought suggests some obvious candidates for extremality

here: all complete ( r −  1)-partite graphs are edge-maximal without containing  Kr. But which among these have the greatest number of edges?

Clearly those whose partition sets are as equal as possible, i.e. differ in size by at most 1: if  V 1 , V 2 are two partition sets with  |V 1 | − |V 2 | > 2, we may increase the number of edges in our complete ( r −  1)-partite graph by moving a vertex from  V 1 across to  V 2.

The unique complete ( r −  1)-partite graphs on  n >  r −  1 vertices whose partition sets differ in size by at most 1 are called  Tur´

an graphs;

we denote them by  T r− 1( n) and their number of edges by  tr− 1( n) T r− 1( n)

(Fig. 7.1.2). For  n < r −  1 we shall formally continue to use these tr− 1( n)

definitions, with the proviso that—contrary to our usual terminology—

the partition sets may now be empty; then, clearly,  T r− 1( n) =  Kn  for all  n  6  r −  1.

Fig. 7.1.2.  The Turán graph  T  3(8)

The following theorem tells us that  T r− 1( n) is indeed extremal for n  and  Kr, and as such unique; in particular, ex( n, Kr) =  tr− 1( n).

150

7. Substructures in Dense Graphs

Theorem 7.1.1. (Turán 1941)

[ 7.1.2 ]

For all integers r, n with r >  1 , every graph G 6⊇ Kr with n vertices and

[ 9.2.2 ]

ex( n, Kr)  edges is a T r− 1( n) .

Proof . We apply induction on  n. For  n  6  r −  1 we have  G =  Kn =

T r− 1( n) as claimed. For the induction step, let now  n >  r.

Since  G  is edge-maximal without a  Kr  subgraph,  G  has a sub-K

graph  K =  Kr− 1. By the induction hypothesis,  G − K  has at most tr− 1( n − r + 1) edges, and each vertex of  G − K  has at most  r −  2

neighbours in  K. Hence,

µ

¶

k

r −  1

Gk  6  tr− 1( n − r + 1) + ( n − r + 1)( r −  2) +

=  t

2

r− 1( n) ;

(1)

the equality on the right follows by inspection of the Turán graph  T r− 1( n) (Fig. 7.1.3).

r −  2

¡

¢

r− 1

2

tr− 1( n − r + 1)

Fig. 7.1.3.  The equation from (1) for  r = 5 and  n = 14

Since  G  is extremal for  Kr (and  T r− 1( n)  6⊇ Kr), we have equality in (1). Thus, every vertex of  G − K  has  exactly r −  2 neighbours in  K—

x 1 , . . . , xr− 1 just like the vertices  x 1 , . . . , xr− 1 of  K  itself. For  i = 1 , . . . , r −  1 let V 1 , . . . , Vr− 1

Vi :=  { v ∈ V ( G)  | vxi /

∈ E( G)  }

be the set of all vertices of  G  whose  r −  2 neighbours in  K  are precisely the vertices other than  xi. Since  Kr 6⊆ G, each of the sets  Vi  is independent, and they partition  V ( G). Hence,  G  is ( r −  1)-partite. As  T r− 1( n) is the unique ( r −  1)-partite graph with  n  vertices and the maximum number of edges, our claim that  G =  T r− 1( n) follows from the assumed extremality of  G.

¤

The Turán graphs  T r− 1( n) are dense: in order of magnitude, they have about  n 2 edges. More exactly, for every  n  and  r  we have r −  2

tr− 1( n) 6 1  n 2

,

2

r −  1

7.1 Subgraphs

151

with equality whenever  r −  1 divides  n (Exercise 8). It is therefore remarkable that just  ²n 2 more edges (for any fixed  ² >  0 and  n  large) give us not only a  Kr  subgraph (as does Turán’s theorem) but a  Krs  for any given integer  s—a graph itself teeming with  Kr  subgraphs: Theorem 7.1.2. (Erd˝

os & Stone 1946)

For all integers r > 2  and s > 1 , and every ² >  0 , there exists an integer n 0  such that every graph with n >  n 0  vertices and at least tr− 1( n) +  ²n 2

edges contains Krs as a subgraph.

We shall prove this theorem in Section 7.3.

The Erd˝

os-Stone theorem is interesting not only in its own right: it

also has a most interesting corollary. In fact, it was this entirely unexpected corollary that established the theorem as a kind of meta-theorem for the extremal theory of dense graphs, and thus made it famous.

Given a graph  H  and an integer  n, consider the number  hn :=

¡ ¢

ex( n, H) / n : the maximum edge density that an  n-vertex graph can 2

have without containing a copy of  H. Could it be that this critical density is essentially just a function of  H, that  hn  converges as  n → ∞?

Theorem 7.1.2 implies this, and more: the limit of  hn  is determined by a very simple function of a natural invariant of  H—its chromatic number!

Corollary 7.1.3.  For every graph H with at least one edge,

µ ¶ −

n

1

χ( H)  −  2

lim ex( n, H)

=

.

n→∞

2

χ( H)  −  1

For the proof of Corollary 7.1.3 we need as a lemma that  tr− 1( n) never deviates much from the value it takes when  r −  1 divides  n (see

¡ ¢

above), and that  t

n

r− 1( n) /

converges accordingly. The proof of the

2

lemma is left as an easy exercise with hint (Exercise 9).

Lemma 7.1.4.

[ 7.1.2 ]

µ ¶ −

n

1

r −  2

lim  tr− 1( n)

=

.

n→∞

2

r −  1

¤

152

7. Substructures in Dense Graphs

r

Proof of Corollary 7.1.3. Let  r :=  χ( H). Since  H  cannot be coloured with  r −  1 colours, we have  H 6⊆ T r− 1( n) for all  n ∈  N, and hence tr− 1( n) 6 ex( n, H)  .

On the other hand,  H ⊆ Krs  for all sufficiently large  s, so ex( n, H) 6 ex( n, Krs)

s

for all those  s. Let us fix such an  s. For every  ² >  0, Theorem 7.1.2

implies that eventually (i.e. for large enough  n)

ex( n, Krs)  < tr− 1( n) +  ²n 2 .

Hence for  n  large,

¡ ¢

¡ ¢

t

n

n

r− 1( n) /

6 ex( n, H) /

2

2

¡ ¢

6 ex( n, Kr

n

s ) /  2

¡ ¢

¡ ¢

< t

n

n

r− 1( n) /

+  ²n 2 /

2

2

¡ ¢

=  t

n

r− 1( n) /

+ 2 ²/(1  −  1 )

2

n

¡ ¢

6  t

n

r− 1( n) /

+ 4 ²

(assume  n > 2).

2

¡ ¢

Therefore, since  t

n

r− 1( n) /

converges to  r− 2 (Lemma 7.1.4), so does

¡ ¢

2

r− 1

ex( n, H) / n . Thus

2

µ ¶ −

n

1

r −  2

lim ex( n, H)

=

n→∞

2

r −  1

as claimed.

¤

For bipartite graphs  H, Corollary 7.1.3 says that substantially fewer

¡ ¢

than  n  edges suffice to force an  H  subgraph. It turns out that 2

c 1 n 2 −  2

r+1

6 ex( n, Kr,r) 6  c 2 n 2 − 1 r

for suitable constants  c 1 , c 2 depending on  r; the lower bound is obtained by random graphs,2 the upper bound is calculated in Exercise 13. If  H

is a forest, then  H ⊆ G  as soon as  ε( G) is large enough, so ex( n, H) is at most linear in  n (Exercise 5). Erd˝

os and Sós conjectured in 1963

that ex( n, T ) 6 1 ( k −  1) n  for all trees with  k > 2 edges; as a general 2

bound for all  n, this is best possible for every  T . See Exercises 15–18 for details.

2 see Chapter 11

7.2 Szemerédi’s regularity lemma

153

7.2 Szemerédi’s regularity lemma

More than 20 years ago, in the course of the proof of a major result on the Ramsey properties of arithmetic progressions, Szemerédi developed a graph theoretical tool whose fundamental importance has been realized

more and more in recent years: his so-called  regularity  or  uniformity lemma. Very roughly, the lemma says that all graphs can be approximated by random graphs in the following sense: every graph can be

partitioned, into a bounded number of equal parts, so that most of its

edges run between different parts and the edges between any two parts

are distributed fairly uniformly—just as we would expect it if they had been generated at random.

In order to state the regularity lemma precisely, we need some defi-

nitions. Let  G = ( V, E) be a graph, and let  X, Y ⊆ V  be disjoint. Then we denote by  kX, Y k  the number of  X– Y  edges of  G, and call kX, Y k

kX, Y k

d( X, Y ) :=  |X||Y |

d( X, Y )

the  density  of the pair ( X, Y ). (This is a real number between 0 and 1.) density

Given some  ² >  0, we call a pair ( A, B) of disjoint sets  A, B ⊆ V ²-regular if all  X ⊆ A  and  Y ⊆ B  with

²-regular

pair

|X| >  ² |A|  and  |Y | >  ² |B|

satisfy

¯¯

¯

d( X, Y )  − d( A, B)¯ 6  ² .

The edges in an  ²-regular pair are thus distributed fairly uniformly: the smaller  ², the more uniform their distribution.

Consider a partition  { V 0 , V 1 , . . . , Vk }  of  V  in which one set  V 0 has been singled out as an  exceptional set . (This exceptional set  V 0 may  exceptional set

be empty.3) We call such a partition an  ²-regular  partition of  G  if it satisfies the following three conditions:

(i)  |V 0 |  6  ² |V |;

²-regular

partition

(ii)  |V 1 | =  . . . =  |Vk|;

(iii) all but at most  ²k 2 of the pairs ( Vi, Vj) with 1 6  i < j  6  k  are ²-regular.

The role of the exceptional set  V 0 is one of pure convenience: it makes it possible to require that all the other partition sets have exactly the same size. Since condition (iii) affects only the sets  V 1 , . . . , Vk, we 3 So  V 0 may be an exception also to our terminological rule that partition sets are not normally empty.

154

7. Substructures in Dense Graphs

may think of  V 0 as a kind of bin: its vertices are disregarded when the uniformity of the partition is assessed, but there are only few such vertices.

Lemma 7.2.1. (Regularity Lemma)

[ 9.2.2 ]

For every ² >  0  and every integer m > 1  there exists an integer M

such that every graph of order at least m admits an ²-regular partition

{ V 0 , V 1 , . . . , Vk } with m  6  k  6  M.

The regularity lemma thus says that, given any  ² >  0, every graph has an  ²-regular partition into a bounded number of sets. The upper bound  M  on the number of partition sets ensures that for large graphs the partition sets are large too; note that  ²-regularity is trivial when the partition sets are singletons, and a powerful property when they are large. In addition, the lemma allows us to specify a lower bound  m  on the number of partition sets; by choosing  m  large, we may increase the proportion of edges running between different partition sets (rather than inside one), i.e. the proportion of edges that are subject to the regularity assertion.

Note that the regularity lemma is designed for use with dense

graphs:4 for sparse graphs it becomes trivial, because all densities of pairs—and hence their differences—tend to zero (Exercise 22).

The remainder of this section is devoted to the proof of the regu-

larity lemma. Although the proof is not difficult, a reader meeting the regularity lemma here for the first time is likely to draw more insight from seeing how the lemma is typically applied than from studying the

technicalities of its proof. Any such reader is encouraged to skip to the start of Section 7.3 now and come back to the proof at his or her leisure.

We shall need the following inequality for reals  µ 1 , . . . , µk >  0 and e 1 , . . . , ek > 0:

X

P

e 2

e

i > (

i)2

P

.

(1)

µi

µi

P

P

P

This follows from the Cauchy-Schwarz inequality

a 2

b 2 > (

a

i

i

ibi)2

√

√

by taking  ai :=

µi  and  bi :=  ei/ µi.

G = ( V, E)

Let  G = ( V, E) be a graph and  n :=  |V |. For disjoint sets  A, B ⊆ V

n

we define

|A| |B|

kA, Bk 2

q( A, B)

q( A, B) :=

d 2( A, B) =

.

n 2

|A| |B| n 2

For partitions  A  of  A  and  B  of  B  we set X

q( A, B)

q( A, B) :=

q( A0, B0)  ,

A0 ∈A;  B0 ∈B

4 Sparse versions do exist, though; see the notes.

7.2 Szemerédi’s regularity lemma

155

and for a partition  P =  { C 1 , . . . , Ck }  of  V  we let X

q( P) :=

q( Ci, Cj)  .

q( P)

i<j

However, if  P =  { C 0 , C 1 , . . . , Ck }  is a partition of  V with exceptional set C 0, we treat  C 0 as a set of singletons and define q( P) :=  q( ˜

P)  ,

©

ª ©

ª

where ˜

P :=  C 1 , . . . , Ck ∪ { v } :  v ∈ C 0 .

˜

P

The function  q( P) plays a pivotal role in the proof of the regularity lemma. On the one hand, it measures the uniformity of the partition  P: if  P  has too many irregular pairs ( A, B), we may take the pairs ( X, Y ) of subsets violating the regularity of the pairs ( A, B) and make those sets X  and  Y  into partition sets of their own; as we shall prove, this refines P  into a partition for which  q  is substantially greater than for  P. Here,

‘substantial’ means that the increase of  q( P) is bounded below by some constant depending only on  ². On the other hand,

X

q( P) =

q( Ci, Cj)

i<j

X  |C

=

i| |Cj | d 2( C

n 2

i, Cj )

i<j  X

6 1

|C

n 2

i| |Cj |

i<j

6 1  .

The number of times that  q( P) can be increased by a constant is thus also bounded by a constant—in other words, after some bounded number

of refinements our partition will be  ²-regular! To complete the proof of the regularity lemma, all we have to do then is to note how many sets

that last partition can possibly have if we start with a partition into  m sets, and to choose this number as our desired bound  M .

Let us make all this precise. We begin by showing that, when we

refine a partition, the value of  q  will not decrease:

Lemma 7.2.2.

(i)  Let C, D ⊆ V be disjoint. If C is a partition of C and D is a

partition of D, then q( C, D) >  q( C, D) .

(ii)  If P, P0 are partitions of V and P0 refines P, then q( P0) >  q( P) .

156

7. Substructures in Dense Graphs

Proof . (i) Let  C =:  { C 1 , . . . , Ck }  and  D =:  { D 1 , . . . , D` }. Then X

q( C, D) =

q( Ci, Dj)

i,j

1 X  kCi, Djk 2

=  n 2

|Ci| |Dj|

i,j

¡ P

¢

k

2

C

> 1

i,j

i, Dj k

P

(1)  n 2

|C

i,j

i| |Dj |

1

kC, Dk 2

=

¡ P

¢¡ P

¢

n 2

|C

|D

i

i|

j

j |

=  q( C, D)  .

(ii) Let  P =:  { C 1 , . . . , Ck }, and for  i = 1 , . . . , k  let  Ci  be the partition of  Ci  induced by  P0. Then

X

q( P) =

q( Ci, Cj)

i<j

X

6

q( Ci, Cj)

(i)  i<j

6  q( P0)  ,

P

P

since  q( P0) =

q( C

q( C

i

i) +

i<j

i, Cj ).

¤

Next, we show that refining a partition by subpartitioning an ir-

regular pair of partition sets increases the value of  q  a little; since we are dealing here with a single pair only, the amount of this increase will still be less than any constant.

Lemma 7.2.3.  Let ² >  0 , and let C, D ⊆ V be disjoint. If ( C, D)  is not ²-regular, then there are partitions C = ( C 1 , C 2)  of C and D = ( D 1 , D 2) of D such that

|C| |D|

q( C, D) >  q( C, D) +  ² 4

.

n 2

Proof . Suppose ( C, D) is not  ²-regular. Then there are sets  C 1  ⊆ C  and D 1  ⊆ D  with  |C 1 | > ² |C|  and  |D 1 | > ² |D|  such that

|η| > ²

(2)

η

for  η :=  d( C 1 , D 1)  − d( C, D). Let  C :=  { C 1 , C 2  }  and  D :=  { D 1 , D 2  }, where  C 2 :=  C  r  C 1 and  D 2 :=  D  r  D 1.

7.2 Szemerédi’s regularity lemma

157

Let us show that  C  and  D  satisfy the conclusion of the lemma. We shall write  ci :=  |Ci|,  di :=  |Di|,  eij :=  kCi, Djk,  c :=  |C|,  d :=  |D|

ci, di, eij

and  e :=  kC, Dk. As in the proof of Lemma 7.2.2,

c, d, e

1 X  e 2

q( C, D) =

ij

n 2

cidj

i,j

µ

¶

1

e 2

X  e 2

=

11 +

ij

n 2

c 1 d 1

cidj

i+ j>  2

µ

¶

> 1

e 211

( e − e

+

11)2

.

(1)  n 2

c 1 d 1

cd − c 1 d 1

By definition of  η, we have  e 11 =  c 1 d 1 e/cd +  ηc 1 d 1, so µ

¶

c

2

n 2  q( C, D) >

1

1 d 1 e +  ηc

c

1 d 1

1 d 1

cd

µ

¶

1

cd − c

2

+

1 d 1  e − ηc

cd − c

1 d 1

1 d 1

cd

c

2 eηc

=

1 d 1 e 2 +

1 d 1 +  η 2 c

c 2 d 2

cd

1 d 1

cd − c

η 2 c 2

+

1 d 1  e 2  −  2 eηc 1 d 1 +

1 d 2

1

c 2 d 2

cd

cd − c 1 d 1

>  e 2 +  η 2 c

cd

1 d 1

>  e 2 +  ² 4 cd

(2)  cd

since  c 1 >  ²c  and  d 1 >  ²d  by the choice of  C 1 and  D 1.

¤

Finally, we show that if a partition has enough irregular pairs of

partition sets to fall short of the definition of an  ²-regular partition, then subpartitioning all those pairs at once results in an increase of  q  by a constant:

Lemma 7.2.4.  Let  0  < ²  6 1 / 4 , and let P =  { C 0 , C 1 , . . . , Ck }

be a partition of V , with exceptional set C 0  of size |C 0 |  6  ²n and

|C 1 | =  . . . =  |Ck| =:  c. If P is not ²-regular, then there is a partition c

P0 =  { C0

}

0 , C 0 1 , . . . , C 0

of V with exceptional set C0

`

0 , where k  6  `  6  k 4 k ,

such that |C0 |  6  |

0

C 0 | +  n/ 2 k, all other sets C0 have equal size, and i

q( P0) >  q( P) +  ² 5 / 2  .

158

7. Substructures in Dense Graphs

Cij

Proof . For all 1 6  i < j  6  k, let us define a partition  Cij  of  Ci  and a partition  Cji  of  Cj, as follows. If the pair ( Ci, Cj) is  ²-regular, we let Cij :=  { Ci }  and  Cji :=  { Cj }. If not, then by Lemma 7.2.3 there are partitions  Cij  of  Ci  and  Cji  of  Cj  with  |Cij| =  |Cji| = 2 and

|C

² 4 c 2

q( C

i| |Cj |

ij , Cji) >  q( Ci, Cj ) +  ² 4

=  q( C

.

(3)

n 2

i, Cj ) +

n 2

Ci

For each  i = 1 , . . . , k, let  Ci  be the unique minimal partition of  Ci  that refines every partition  Cij  with  j 6=  i. (In other words, if we consider two elements of  Ci  as equivalent whenever they lie in the same partition set of  Cij  for every  j 6=  i, then  Ci  is the set of equivalence classes.) Thus,

|Ci|  6 2 k− 1. Now consider the partition

k

[

C

C :=  { C 0  } ∪

Ci

i=1

of  V , with  C 0 as exceptional set. Then  C  refines  P, and k  6  |C|  6  k 2 k.

(4)

©

ª

C 0

Let  C 0 :=  { v } :  v ∈ C 0 . Now if  P  is not  ²-regular, then for more than  ²k 2 of the pairs ( Ci, Cj) with 1 6  i < j  6  k  the partition  Cij  is non-trivial. Hence, by our definition of  q  for partitions with exceptional set, and Lemma 7.2.2 (i),

X

X

X

q( C) =

q( Ci, Cj) +

q( C 0 , Ci) +

q( Ci)

16 i<j

16 i

06 i

X

X ¡

¢

>

q( Cij, Cji) +

q C 0 , { Ci } +  q( C 0)

16 i<j

16 i

X

X ¡

¢

>

² 4 c 2

q( Ci, Cj) +  ²k 2

+

q C 0 , { Ci } +  q( C 0)

(3)

n 2

16 i<j

16 i

µ ¶

kc  2

=  q( P) +  ² 5

n

>  q( P) +  ² 5 / 2  .

(For the last inequality, recall that  |C 0 |  6  ²n  6 1  n, so  kc > 3  n.) 4

4

In order to turn  C  into our desired partition  P0, all that remains to do is to cut its sets up into pieces of some common size, small enough that all remaining vertices can be collected into the exceptional set without making this too large. Let  C0 1 , . . . , C0  be a maximal collection of dis-

`

d

joint sets of size  d :=  bc/ 4 kc  such that each  C0  is contained in some i

7.2 Szemerédi’s regularity lemma

159

S

C ∈ C  r  { C 0  }, and put  C0

}

0 :=  V  r

C0. Then  P0 =  { C0

P0

i

0 , C 0 1 , . . . , C 0`

is indeed a partition of  V . Moreover, ˜

P0  refines ˜

C, so

q( P0) >  q( C) >  q( P) +  ² 5 / 2

by Lemma 7.2.2 (ii). Since each set  C0 6=  C0

i

0 is also contained in one

of the sets  C 1 , . . . , Ck, but no more than 4 k  sets  C0  can lie inside the i

same  Cj (by the choice of  d), we also have  k  6  `  6  k 4 k  as required.

Finally, the sets  C0 1 , . . . , C0  use all but at most  d  vertices from each set

`

C 6=  C 0 of  C. Hence,

|C0 |  6  |

0

C 0 | +  d |C|

6  |

c

C 0 | +

k 2 k

(4)

4 k

=  |C 0 | +  ck/ 2 k

6  |C 0 | +  n/ 2 k.

¤

The proof of the regularity lemma now follows easily by repeated

application of Lemma 7.2.4:

Proof of Lemma 7.2.1. Let  ² >  0 and  m > 1 be given; without loss ², m

of generality,  ²  6 1 / 4. Let  s := 2 /² 5. This number  s  is an upper bound s

on the number of iterations of Lemma 7.2.4 that can be applied to a partition of a graph before it becomes  ²-regular; recall that  q( P) 6 1 for all partitions  P.

There is one formal requirement which a partition  { C 0 , C 1 , . . . , Ck }

with  |C 1 | =  . . . =  |Ck|  has to satisfy before Lemma 7.2.4 can be (re-) applied: the size  |C 0 |  of its exceptional set must not exceed  ²n. With each iteration of the lemma, however, the size of the exceptional set can grow by up to  n/ 2 k. (More precisely, by up to  n/ 2 `, where  `  is the number of other sets in the current partition; but  ` >  k  by the lemma, so  n/ 2 k  is certainly an upper bound for the increase.) We thus want to choose  k  large enough that even  s  increments of  n/ 2 k  add up to at most 1  ²n, and  n  large enough that, for any initial value of  |C

2

0 | < k, we

have  |C 0 |  6 1  ²n. (If we give our starting partition  k  non-exceptional 2

sets  C 1 , . . . , Ck, we should allow an initial size of up to  k  for  C 0, to be able to achieve  |C 1 | =  . . . =  |Ck|.)

So let  k >  m  be large enough that 2 k− 1 >  s/². Then  s/ 2 k  6  ²/ 2, k

and hence

s

k +

n  6  ²n

(5)

2 k

whenever  k/n  6  ²/ 2, i.e. for all  n > 2 k/².

Let us now choose  M . This should be an upper bound on the

number of (non-exceptional) sets in our partition after up to  s  iterations

160

7. Substructures in Dense Graphs

of Lemma 7.2.4, where in each iteration this number may grow from its current value  r  to at most  r 4 r. So let  f  be the function  x 7→ x 4 x, and M

take  M := max  { f s( k) ,  2 k/² }; the second term in the maximum ensures that any  n >  M  is large enough to satisfy (5).

We finally have to show that every graph  G = ( V, E) of order at least  m  has an  ²-regular partition  { V 0 , V 1 , . . . , Vk }  with  m  6  k  6  M . So n

let  G  be given, and let  n :=  |G|. If  n  6  M , we partition  G  into  k :=  n singletons, choosing  V 0 :=  ∅  and  |V 1 | =  . . . =  |Vk| = 1. This partition of G  is clearly  ²-regular. Suppose now that  n > M . Let  C 0  ⊆ V  be minimal such that  k  divides  |V  r  C 0 |, and let  { C 1 , . . . , Ck }  be any partition of V  r  C 0 into sets of equal size. Then  |C 0 | < k, and hence  |C 0 |  6  ²n  by (5).

Starting with  { C 0 , C 1 , . . . , Ck }  we apply Lemma 7.2.4 again and again, until the partition of  G  obtained is  ²-regular; this will happen after at most  s  iterations, since by (5) the size of the exceptional set in the partitions stays below  ²n, so the lemma could indeed be reapplied up to the theoretical maximum of  s  times.

¤

7.3 Applying the regularity lemma

The purpose of this section is to illustrate how the regularity lemma

is typically applied in the context of (dense) extremal graph theory.

Suppose we are trying to prove that a certain edge density of a graph  G

suffices to force the occurrence of some given subgraph  H, and that we have an  ²-regular partition of  G. The edges inside almost all the pairs ( Vi, Vj) of partition sets are distributed uniformly, although their density may depend on the pair. But since  G  has many edges, this density cannot be zero for all the pairs: some sizeable proportion of the pairs will have positive density. Now if  G  is large, then so are the pairs: recall that the number of partition sets is bounded, and they have equal size. But

any large enough bipartite graph with equal partition sets, fixed positive edge density (however small!) and a uniform distribution of edges will

contain any given bipartite subgraph5—this will be made precise below.

Thus if enough pairs in our partition of  G  have positive density that  H

can be written as the union of bipartite graphs each arising in one of

those pairs, we may hope that  H ⊆ G  as desired.

These ideas will be formalized by Lemma 7.3.2 below. We shall then use this and the regularity lemma to prove the Erd˝

os-Stone theorem

from Section 7.1; another application will be given later, in the proof of

Theorem 9.2.2.

Before we state Lemma 7.3.2, let us note a simple consequence of

the  ²-regularity of a pair ( A, B): for any subset  Y ⊆ B  that is not too 5 Readers already acquainted with random graphs may find it instructive to compare this statement with Proposition 11.3.1.

7.3 Applying the regularity lemma

161

small, most vertices of  A  have about the expected number of neighbours in  Y :

Lemma 7.3.1.  Let ( A, B)  be an ²-regular pair, of density d say, and let Y ⊆ B have size |Y | >  ² |B|. Then all but at most ² |A| of the vertices in A have (each) at least ( d − ²) |Y | neighbours in Y .

Proof . Let  X ⊆ A  be the set of vertices with fewer than ( d − ²) |Y |

neighbours in  Y . Then  kX, Y k < |X|( d − ²) |Y |, so kX, Y k

d( X, Y ) =  |X||Y | < d−² =  d( A,B) −².

Since ( A, B) is  ²-regular, this implies that  |X| < ² |A|.

¤

Let  G  be a graph with an  ²-regular partition  { V 0 , V 1 , . . . , Vk }, with exceptional set  V 0 and  |V 1 | =  . . . =  |Vk| =:  `. Given  d ∈ (0 ,  1 ], let  R  be R

the graph with vertices  V 1 , . . . , Vk  in which two vertices are adjacent if and only if they form an  ²-regular pair in  G  of density >  d. We shall call R  a  regularity graph  of  G  with parameters  ²,  `  and  d. Given  s ∈  N, let regularity

graph

us now replace every vertex  Vi  of  R  by a set  V s  of  s  vertices, and every i

V s

i

edge by a complete bipartite graph between the corresponding  s-sets.

The resulting graph will be denoted by  Rs. (For  R =  Kr, for example, Rs

we have  Rs =  Krs.)

The following lemma says that subgraphs of  Rs  can also be found in  G, provided that  ²  is small enough and the  Vi  are large enough. In fact, the values of  ²  and  `  required depend only on ( d  and) the maximum degree of the subgraph:

Lemma 7.3.2.  For all d ∈ (0 ,  1 ]  and ∆ > 1  there exists an ² 0  >  0  with

[ 9.2.2 ]

the following property: if G is any graph, H is a graph with ∆( H) 6 ∆ , s ∈  N , and R is any regularity graph of G with parameters ²  6  ² 0 ,

` >  s/² 0  and d, then

H ⊆ Rs ⇒ H ⊆ G .

Proof . Given  d  and ∆, choose  ² 0  < d  small enough that d, ∆

∆ + 1

²

( d − ²

0 6 1 ;

(1)

² 0

0)∆

such a choice is possible, since (∆ + 1) ²/( d − ²)∆  →  0 as  ² →  0. Now let  G, H, R, Rs G,  H,  s  and  R  be given as stated. Let  { V 0 , V 1 , . . . , Vk }  be the  ²-regular Vi

partition of  G  that gave rise to  R; thus,  ²  6  ² 0,  V ( R) =  { V 1 , . . . , Vk }

², k, `

and  |V 1 | =  . . . =  |Vk| =  `. Let us assume that  H  is actually a subgraph

162

7. Substructures in Dense Graphs

ui, h

of  Rs (not just isomorphic to one), with vertices  u 1 , . . . , uh  say. Each σ

vertex  ui  lies in one of the  s-sets  V s  of  R

j

s; this defines a map  σ:  i 7→ j.

vi

Our aim is to define an embedding  ui 7→ vi ∈ Vσ( i) of  H  in  G; thus, v 1 , . . . , vh  will be distinct, and  vivj  will be an edge of  G  whenever  uiuj is an edge of  H.

Our plan is to choose the vertices  v 1 , . . . , vh  inductively. Throughout the induction, we shall have a ‘target set’  Yi ⊆ Vσ( i) assigned to each  i; this contains the vertices that are still candidates for the choice of  vi.

Initially,  Yi  is the entire set  Vσ( i). As the embedding proceeds,  Yi  will get smaller and smaller (until it collapses to  { vi }  when  vi  is chosen): whenever we choose a vertex  vj  with  j < i  and  ujui ∈ E( H), we delete all those vertices from  Yi  that are not adjacent to  vj. The set  Yi  thus evolves as

Vσ( i) =  Y  0  ⊇

i

. . . ⊇ Y ii =  { vi } ,

where  Y j  denotes the version of  Y

i

i  current after the definition of  vj (and

any corresponding deletion of vertices from  Y j− 1).

i

In order to make this approach work, we have to ensure that the

target sets  Yi  do not get too small. When we come to embed a vertex  uj, we consider all the indices  i > j  with  ujui ∈ E( H); there are at most ∆

such  i. For each of these  i, we wish to select  vj  so that Y j =  N ( v

(2)

i

j )  ∩ Y j− 1

i

is large, i.e. not much smaller than  Y j− 1. Now this can be done by i

Lemma 7.3.1 (with  A =  Vσ( j),  B =  Vσ( i) and  Y =  Y j− 1): unless  Y j− 1

i

i

is tiny (of size less than  ²`), all but at most  ²`  choices of  vj  will be such that (2) implies

|Y j| > ( d − ²) |Y j− 1 | .

(3)

i

i

Doing this simultaneously for all of the at most ∆ values of  i  considered, we find that all but at most ∆ ²`  choices of  vj  from  Vσ( j), and in particular from  Y j− 1  ⊆ V

j

σ( j), satisfy (3) for all  i.

It remains to show that the sets  Y  considered for Lemma 7.3.1 above are indeed never tiny, and that  |Y j− 1 |− ∆ ²` >  s  to ensure that a suitable j

choice for  vj  exists: since  σ( j0) =  σ( j) for at most  s −  1 of the vertices uj0  with  j0 < j, a choice between  s  suitable candidates for  vj  will suffice to keep  vj  distinct from  v 1 , . . . , vj− 1. But all this follows from our choice of  ² 0. Indeed, the initial target sets  Y  0 have size  `, and each  Y

i

i  has

vertices deleted from it only when some  vj  with  j < i  and  ujui ∈ E( H) is defined, which happens at most ∆ times. Thus,

|Y j| − ∆ ²` > ( d − ²)∆ ` − ∆ ²` > ( d − ²

²

i

0)∆ ` − ∆ ² 0 ` >

0 ` >  s

(3)

(1)

whenever  j < i, so in particular  |Y j| >  ²

| − ∆ ²` >  s.

i

0 ` >  ²`  and  |Y j− 1

j

¤

7.3 Applying the regularity lemma

163

We are now ready to prove the Erd˝

os-Stone theorem.

Proof of Theorem 7.1.2. Let  r > 2 and  s > 1 be given as in the

(7.1.1)

(7.1.4)

statement of the theorem. For  s = 1 the assertion follows from Turán’s r, s

theorem, so we assume that  s > 2. Let  γ >  0 be given; this  γ  will play γ

the role of the  ²  of the theorem. Let  G  be a graph with  |G| =:  n  and kGk >  tr− 1( n) +  γn 2 .

kGk

(Thus,  γ <  1.) We want to show that  Kr ⊆

s

G  if  n  is large enough.

Our plan is to use the regularity lemma to show that  G  has a regularity graph  R  dense enough to contain a  Kr  by Tur´

an’s theorem. Then

Rs  contains a  Krs, so we may hope to use Lemma 7.3.2 to deduce that Kr ⊆

s

G.

On input  d :=  γ  and ∆ := ∆( Krs), Lemma 7.3.2 returns an  ² 0  >  0; d, ∆

since the lemma’s assertion about  ² 0 becomes weaker when  ² 0 is made ² 0

smaller, we may assume that

² 0  < γ/ 2  <  1  .

(1)

To apply the regularity lemma, let  m >  1 /γ  and choose  ² >  0 small enough that  ²  6  ² 0 and

m, ²

δ := 2 γ − ² 2  −  4 ² − d −  1  >  0 ; m

δ

this is possible, since 2 γ − d −  1  >  0. On input  ²  and  m, the regularity

m

lemma returns an integer  M . Let us assume that M

n >

M s

.

n

² 0(1  − ²)

Since this number is at least  m, the regularity lemma provides us with an  ²-regular partition  { V 0 , V 1 , . . . , Vk }  of  G, where  m  6  k  6  M ; let k

|V 1 | =  . . . =  |Vk| =:  `. Then

`

n >  k` ,

(2)

and

n − |V

1  − ²

` =

0 | >  n − ²n =  n

>  s

k

M

M

² 0

by the choice of  n. Let  R  be the regularity graph of  G  with parameters R

², `, d  corresponding to the above partition. Since  ²  6  ² 0 and  ` >  s/² 0, the regularity graph  R  satisfies the premise of Lemma 7.3.2, and by definition of ∆ we have ∆( Krs) = ∆. Thus in order to conclude by Lemma 7.3.2

164

7. Substructures in Dense Graphs

that  Kr ⊆

s

G, all that remains to be checked is that  Kr ⊆ R (and hence Kr ⊆

s

Rs).

Our plan was to show  Kr ⊆ R  by Turán’s theorem. We thus have to check that  R  has enough edges, i.e. that enough  ²-regular pairs ( Vi, Vj) have density at least  d. This should follow from our assumption that  G

has at least  tr− 1( n) +  γn 2 edges, i.e. an edge density of about  r− 2 + 2 γ: r− 1

this lies substantially above the approximate edge density  r− 2 of the r− 1

Turán graph  T r− 1( k), and hence substantially above any density that G  could have if no more than  tr− 1( k) of the pairs ( Vi, Vj) had density

>  d—even if all those pairs had density 1!

Let us then estimate  kRk  more precisely. How many edges of  G

¡

¢

lie outside  ²-regular pairs? At most  |V 0 |  edges lie inside  V

2

0, and by

condition (i) in the definition of  ²-regularity these are at most 1 ( ²n)2

2

edges. At most  |V 0 |k`  6  ²nk`  edges join  V 0 to other partition sets. The at most  ²k 2 other pairs ( Vi, Vj) that are not  ²-regular contain at most

` 2 edges each, together at most  ²k 2 ` 2. The  ²-regular pairs of insufficient density ( < d) each contain no more than  d` 2 edges, altogether at most

¡ ¢

1  k 2 d` 2 edges. Finally, there are at most  `  edges inside each of the 2

2

partition sets  V 1 , . . . , Vk, together at most 1  ` 2 k  edges. All  other  edges 2

of  G  lie in  ²-regular pairs of density at least  d, and thus contribute to edges of  R. Since each edge of  R  corresponds to at most  ` 2 edges of  G, we thus have in total

kGk ≤  1 ² 2 n 2 +  ²nk` +  ²k 2 ` 2 + 1 k 2 d` 2 + 1 ` 2 k +  kRk ` 2 .

2

2

2

Hence, for all sufficiently large  n,

kGk −  1 ² 2 n 2  − ²nk` − ²k 2 ` 2  −  1 dk 2 ` 2  −  1 k` 2

kRk ≥  1 k 2

2

2

2

2

1  k 2 ` 2

2

µ

¶

t

² 2 n 2  − ²nk`

≥  1

r− 1( n) +  γn 2  −  1

k 2

2

−  2 ² − d −  1

2

(1 ,  2)

n 2 / 2

k

µ

¶

≥

t

1  k 2

r− 1( n) + 2 γ − ² 2  −  4 ² − d −  1

2

(2)

n 2 / 2

m

µ

µ ¶ −  µ

¶

¶

n

1

= 1  k 2  t

1  −  1

+  δ

2

r− 1( n)

2

n

r −  2

>  1  k 2

2

r −  1

>  tr− 1( k)  .

(The strict inequality follows from Lemma 7.1.4.) Therefore  Kr ⊆ R  by

Theorem 7.1.1, as desired.

¤

Exercises

165

Exercises

1.  −  Show that  K 1 ,  3 is extremal without a  P  3.

2.  −  Given  k >  0, determine the extremal graphs of chromatic number at most  k.

3.

Determine the value of ex( n, K 1 ,r) for all  r, n ∈  N.

4.

Is there a graph that is edge-maximal without a  K 3 minor but not extremal?

5.

Show that, for every forest  F , the value of ex( n, F ) is bounded above by a linear function of  n.

6.+ Given  k >  0, determine the extremal graphs without a matching of size  k.

(Hint. Theorem 2.2.3 and Ex. 10, Ch. 2.)

7.

Without using Tur´

an’s theorem, show that the maximum number of

edges in a triangle-free graph of order  n >  1 is  bn 2 / 4 c.

8.

Show that

tr− 1( n) 6 1  n 2  r −  2  ,

2

r −  1

with equality whenever  r −  1 divides  n.

¡ ¢

9.

Show that  t

n

r− 1( n) /

converges to ( r −  2) /( r −  1) as  n → ∞.

2

(Hint.  tr− 1(( r −  1) b n c) 6  t

e).)

r− 1

r− 1( n) 6  tr− 1(( r −  1) d n

r− 1

10.+ Given non-adjacent vertices  u, v  in a graph  G, denote by  G [  u → v ] the graph obtained from  G  by first deleting all the edges at  u  and then joining  u  to all the neighbours of  v. Show that  Kr 6⊆ G [  u → v ] if Kr 6⊆ G. Applying this operation repeatedly to a given extremal graph for  n  and  Kr, prove that ex( n, Kr) =  tr− 1( n): in each iteration step, choose  u  and  v  so that the number of edges will not decrease, and so that eventually a complete multipartite graph is obtained.

11.

Show that deleting at most ( m − s)( n − t) /s  edges from a  Km,n  will never destroy all its  Ks,t  subgraphs.

12.

For 0  < s  6  t  6  n  let  z( n, s, t) denote the maximum number of edges in a bipartite graph whose partition sets both have size  n, and which does not contain a  Ks,t. Show that 2 ex( n, Ks,t)  ≤ z( n, s, t)  ≤  ex(2 n, Ks,t).

13.+ Let 1 6  r  6  n  be integers. Let  G  be a bipartite graph with bipartition

{ A, B }, where  |A| =  |B| =  n, and assume that  Kr,r 6⊆ G. Show that X µ

¶

µ ¶

d( x)

6

n

( r −  1)

.

r

r

x∈A

Using the previous exercise, deduce that ex( n, Kr,r) 6  cn 2 − 1 /r  for some constant  c  depending only on  r.

166

7. Substructures in Dense Graphs

14.

The  upper density  of an infinite graph  G  is the infimum of all reals  α

¡ ¢ − 1

such that the finite graphs  H ⊆ G  with  kHk |H|

> α  have bounded

2

order. Show that this number always takes one of the countably many

values 0 ,  1 ,  1  ,  2  ,  3  , . . . .

2

3

4

(Hint. Erd˝

os-Stone.)

15.

Prove the following weakening of the Erd˝

os-S´

os conjecture (stated at

the end of Section 7.1): given integers 2 6  k < n, every graph with  n vertices and at least ( k −  1) n  edges contains every tree with  k  edges as a subgraph.

16.

Show that, as a general bound for arbitrary  n, the bound on ex( n, T ) claimed by the Erd˝

os-S´

os conjecture is best possible for every tree  T .

Is it best possible even for every  n  and every  T ?

17.  −  Prove the Erd˝

os-S´

os conjecture for the case when the tree considered is a star.

18.

Prove the Erd˝

os-S´

os conjecture for the case when the tree considered is a path.

(Hint. Use the result of the next exercise.)

19.

Show that every connected graph  G  contains a path of length at least min  {  2 δ( G) , |G| −  1  }.

20.  −  In the definition of an  ²-regular pair, what is the purpose of the requirement that  |X| > ² |A|  and  |Y | > ² |B|?

21.  −  Show that any  ²-regular pair in  G  is also  ²-regular in  G.

22.

Prove the regularity lemma for sparse graphs, that is, for every sequence ( Gn) n∈ N of graphs  Gn  of order  n  such that  kGnk/n 2  →  0 as  n → ∞.

Notes

The standard reference work for results and open problems in extremal graph theory (in a very broad sense) is still B. Bollobás,  Extremal Graph Theory, Academic Press 1978. A kind of update on the book is given by its author in his chapter of the  Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995. An instructive survey of extremal graph theory in the narrower sense of our chapter is given by M. Simonovits in (L.W. Beineke & R.J. Wilson, eds.)  Selected Topics in Graph Theory 2, Academic Press 1983. This paper focuses among other things on the particular role played by the Turán graphs. A more recent survey by the same author can be found in (R.L. Graham & J. Nešetřil, eds.)  The Mathematics of Paul Erd˝

os, Vol. 2, Springer 1996.

Tur´

an’s theorem is not merely one extremal result among others: it is the result that sparked off the entire line of research. Our proof of Tur´

an’s

theorem is essentially the original one; the proof indicated in Exercise 10 is due to Zykov.

Notes

167

Our version of the Erd˝

os-Stone theorem is a slight simplification of the

original. A direct proof, not using the regularity lemma, is given in L. Lovász, Combinatorial Problems and Exercises (2nd edn.), North-Holland 1993. Its most fundamental application, Corollary 7.1.3, was only found 20 years after the theorem, by Erd˝

os and Simonovits (1966).

Of our two bounds on ex( n, Kr,r) the upper one is thought to give the correct order of magnitude. For vastly off-diagonal complete bipartite graphs this was verified by J. Kollár, L. Rónyai & T. Szabó, Norm-graphs and bipartite Turán numbers,  Combinatorica 16 (1996), 399–406, who proved that ex( n, Kr,s) >  crn 2 −  1 r  when  s > r! .

Details about the Erd˝

os-S´

os conjecture, including an approximate solu-

tion for large  k, can be found in the survey by Komlós and Simonovits cited below. The case where the tree  T  is a path (Exercise 18) was proved by Erd˝

os & Gallai in 1959. It was this result, together with the easy case of stars

(Exercise 17) at the other extreme, that inspired the conjecture as a possible unifying result.

The regularity lemma is proved in E. Szemerédi, Regular partitions of

graphs,  Colloques Internationaux CNRS 260 —Probl`

emes Combinatoires et

Th´

eorie des Graphes, Orsay (1976), 399–401. Our rendering follows an account by Scott (personal communication). A broad survey on the regular-

ity lemma and its applications is given by J. Komlós & M. Simonovits in (D. Miklós, V.T. Sós & T. Sz˝

onyi, eds.)  Paul Erd˝

os is 80, Vol. 2, Proc. Colloq.

Math. Soc. János Bolyai (1996); the concept of a regularity graph and Lemma 7.3.2 are taken from this paper. An adaptation of the regularity lemma for use with sparse graphs was developed independently by Kohayakawa and by Rödl; see Y. Kohayakawa, Szemerédi’s regularity lemma for sparse graphs, in (F. Cucker & M. Shub, eds.)  Foundations of Computational Mathematics, Selected papers of a conference held at IMPA in Rio de Janeiro, January 1997, Springer 1997.

8

Substructures in

Sparse Graphs

In this chapter we study how global assumptions about a graph—on its

average degree, chromatic number, or even (large) girth—can force it to contain a given graph  H  as a minor or topological minor. As we know already from Mader’s theorem 3.6.1, there exists a function  h  such that an average degree of  d( G) >  h( r) suffices to create a  T Kr  subgraph in  G, and hence a (topological)  H  minor if  r >  |H|. Since a graph with  n  vertices and average degree  d  has 1  dn  edges this shows that, for 2

every  H, there is a ‘constant’  c (depending on  H  but not on  n) such that a topological  H  minor occurs in every graph with  n  vertices and at least  cn  edges. Such graphs with a number of edges about linear1 in their order are called  sparse—so this is a chapter about substructures in sparse

sparse graphs.

The first question, then, will be the analogue of Tur´

an’s theorem:

given a positive integer  r, what is the minimum value of the above ‘constant’  c  for  H =  Kr, i.e. the smallest growth rate of a function  h( r) as in Theorem 3.6.1? This was a major open problem until very recently; we present its solution, which builds on some fascinating methods the

problem has inspired over time, in Section 8.1.

If raising the average degree suffices to force the occurrence of a

certain minor, then so does raising any other invariant which in turn

forces up the average degree. For example, if  d( G) >  c  implies  H  4  G, then so will  χ( G) >  c + 1 (by Corollary 5.2.3). However, is this best possible? Even if the value of  c  above is least possible for  d( G) >  c  to imply  H  4  G, it need not be so for  χ( G) >  c + 1 to imply  H  4  G. One of the most famous conjectures in graph theory, the  Hadwiger conjecture,

1 Compare the footnote at the beginning of Chapter 7.

170

8. Substructures in Sparse Graphs

√

suggests that there is indeed a gap here: while a value of  c =  c0r  log  r (where  c0  is independent of both  n  and  r) is best possible for  d( G) >  c to imply  H  4  G (Section 8.2), the conjecture says that  χ( G) >  r  will do the same! Thus, if true, then Hadwiger’s conjecture shows that the effect of a large chromatic number on the occurrence of minors somehow

goes beyond that part which is well-understood: its effect via mere edge density. We shall consider Hadwiger’s conjecture in Section 8.3.

8.1 Topological minors

In this section we prove that an average degree of  cr 2 suffices to force the occurrence of a topological  Kr  minor in a graph; complete bipartite graphs show that, up to the constant  c, this is best possible (Exercise 5).

The following theorem was proved independently around 1996 by

Bollobás & Thomason and by Komlós & Szemerédi.

Theorem 8.1.1.  There exists a c ∈  R  such that, for every r ∈  N , every graph G of average degree d( G) >  cr 2  contains Kr as a topological minor.

The proof of this theorem, in which we follow Bollobás & Thomason,

will occupy us for the remainder of this section. A set  U ⊆ V ( G) will linked

be called  linked (in  G) if for any distinct vertices  u 1 , . . . , u 2 h ∈ U  there are  h  disjoint paths  Pi =  u 2 i− 1  . . . u 2 i  in  G,  i = 1 , . . . , h.2 The graph  G

( k, `) -linked

itself is ( k, `) -linked  if every  k-set of its vertices contains a linked  `-set.

How can we hope to find the  T Kr  in  G  claimed to exist by Theorem 8.1.1? Our basic approach will be to identify first some  r-set  X

as a set of branch vertices, and to choose for each  x ∈ X  a set  Yx  of r −  1 neighbours, one for every edge incident with  x  in the  Kr. If the constant  c  from the theorem is large enough, the  r +  r( r −  1) =  r 2

S

vertices of  X ∪

Yx  can be chosen distinct: by Proposition 1.2.2,  G  has a subgraph of minimum degree at least  ε( G) = 1  d( G) > 1  cr 2, so we 2

2

can choose  X  and its neighbours inside this subgraph. Having fixed  X

and the sets  Yx, we then have to link up the correct pairs of vertices in S

Y :=

Yx  by disjoint paths in  G − X, to obtain the desired  T Kr.

This would be possible at once if  Y  were linked in  G − X. Unfortunately, this is unrealistic to hope for: no average degree, however large, will force  every r( r −  1)-set to be linked. (Why not?) However, if we pick for  X  significantly more than the  r  vertices needed eventually, and for each  x ∈ X  significantly more than  r −  1 neighbours as  Yx, then  Y

might become so large that the high average degree of  G  guarantees the 2 Thus, in a  k-linked graph—see Chapter 3.6—every set of up to 2 k + 1 vertices is linked.

8.1 Topological minors

171

existence of some large linked subset  Z ⊆ Y . This would be the case if G  were ( k, `)-linked for some  k  6  |Y |  and  ` >  |Z|.

As above, a large enough constant  c  will easily ensure that  X  and  Y

can be chosen with many vertices to spare. Another problem, however,

is more serious: it will not be enough to make  ` (and hence  Z) large in absolute terms. Indeed, if  k (and  Y ) is much larger still, it might happen that  Z, although large, consists of neighbours of only a few vertices in  X!

We thus have to ensure that  `  is large also relative to  k. This will be the purpose of our first lemma (8.1.2): it establishes a sufficient condition for  G  to be ( k, dk/ 2 e)-linked.

What is this sufficient condition? It is the assumption that  G  has a particularly dense minor  H, one whose minimum degree exceeds 1  |H|

2

by a positive fraction of  k. (In particular,  H  will be dense in the sense of Chapter 7.) In view of Theorem 3.6.2, it is not surprising that such a dense graph  H  is highly linked. Given sufficiently high connectivity of  G (which again follows easily if  c  is large enough), we may then try to link up the vertices of any  Y  as above to distinct branch sets of  H  by disjoint paths in  G  avoiding most of the other branch sets, and thus to transfer the linking properties of  H  to a  dk/ 2 e-set  Z ⊆ Y (Fig. 8.1.1).

H

Y

Z

X

x 1

x 2

x 3

Fig. 8.1.1.  Finding a  T K 3 in  G  with branch vertices  x 1 , x 2 , x 3

What is all the more surprising, however, is that the existence of

such a dense minor  H  can be deduced from our assumption of  d( G) >  cr 2.

This will be shown in another lemma (8.1.3); the assertion of the theorem itself will then follow easily.

Lemma 8.1.2.  If G is k-connected and has a minor H with  2 δ( H) >

|H| + 3 k, then G is ( k, dk/ 2 e) -linked.

2

172

8. Substructures in Sparse Graphs

(3.3.1)

Proof . Let  V :=  { Vx | x ∈ V ( H)  }  be the set of branch sets in  G

V, Vx

corresponding to the vertices of  H. For our proof that  G  is ( k, dk/ 2 e)-

v 1 , . . . , vk

linked, let  k  distinct vertices  v 1 , . . . , vk ∈ G  be given. Let us call a linkage

sequence  P 1 , . . . , Pk  of disjoint paths in  G  a  linkage  if the  Pi  each start in  vi  and end in pairwise distinct sets  V ∈ V; the paths  Pi  themselves will link

be called  links. Since our assumptions about  H  imply that  |H| >  k, and G  is  k-connected, such linkages exist: just pick  k  vertices from pairwise distinct sets  V ∈ V, and link them disjointly to  { v 1 , . . . , vk }  by Menger’s

theorem.

P

Now let  P = ( P 1 , . . . , Pk) be a linkage whose total number of edges S

P 1 , . . . , Pk

outside

V ∈V G [  V ] is as small as possible. Thus, if  f ( P ) denotes the f ( P )

number of edges of  P  not lying in any  G [  Vx ], we choose  P  so as to P

minimize

k

f ( P

i=1

i). Then for every  V ∈ V  that meets a path  Pi ∈ P

there exists one such path that ends in  V : if not, we could terminate  Pi in  V  and reduce  f ( Pi). Thus, exactly  k  of the branch sets of  H  meet a link. Let us divide these sets into two classes:

U

U :=  { V ∈ V | V  meets exactly one link  }

W

W :=  { V ∈ V | V  meets more than one link  } .

Since  H  is dense and each  U ∈ U  meets only one link, it will be easy to show that the starting vertices  vi  of those links form a linked set in  G.

Hence, our aim is to show that  |U| >  dk/ 2 e, i.e. that  U  is no smaller than  W. (Recall that  |U| +  |W| =  k.) To this end, we first prove the following:

Every V ∈ W is met by some link which leaves V again

(1)

and next meets a set from U (where it ends).

x

Suppose  Vx ∈ W  is a counterexample to (1). Since

2 δ( H) >  |H| + 3  k >  δ( H) + 3  k , 2

2

we have  δ( H) > 3  k. As  |U ∪ W| =  k, this implies that  x  has a neighbour 2

y  in  H  with  Vy ∈ V  r ( U ∪ W); let  wxwy  be an edge of  G  with  wx ∈ Vx and  wy ∈ Vy. Let  Q =  w . . . wxwy  be a path in  G [  Vx ∪ { wy } ] of whose Pi

vertices only  w  lies on any link, say on  Pi (Fig. 8.1.2). Replacing  Pi  in P 0i

P  by  P 0 :=  P

i

iwQ  then yields another linkage.

If  Pi  is not the link ending in  Vx, then  f ( P 0) 6  f ( P

i

i). The choice

of  P  then implies that  f ( P 0) =  f ( P

i

i), i.e. that  Pi  ends in the branch set

W  it enters immediately after  Vx. Since  Vx  is a counterexample to (1) we have  W /

∈ U, i.e.  W ∈ W. Let  P 6=  Pi  be another link meeting  W .

Then  P  does not end in  W (because  Pi  ends there); let  P 0 ⊆ P  be the (minimal) initial segment of  P  that ends in  W . If we now replace  Pi  and P  by  P 0  and  P 0  in  P, we obtain a linkage contradicting the choice of  P.

i

8.1 Topological minors

173

Vy ∈ V  r ( U ∪ W)

V ∈ W

x

wy

P 0i

wx

Pi

w

Pi

P

H

P 0

W ∈ W

Fig. 8.1.2.  If  Pi  does not end in  Vx, we replace  Pi  and  P  by  P 0i and  P 0

We now assume that  Pi  does end in  Vx; then  f ( P 0) =  f ( P

i

i) + 1.

As  Vx ∈ W, there exists a link  Pj  that meets  Vx  and leaves it again; let P 0  be the initial segment of  P

) 6

j

j  ending in  Vx (Fig 8.1.3). Then  f ( P 0

j

f ( Pj)  −  1. In fact, since replacing  Pi  and  Pj  with  P 0  and  P 0  in  P  yields i

j

another linkage, the choice of  P  implies that  f ( P 0) =  f ( P

j

j )  −  1, so  Pj

ends in the branch set  W  it enters immediately after  Vx. Then  W ∈ W

as before, so we may define  P  and  P 0  as before. Replacing  Pi,  Pj  and  P

by  P 0,  P 0  and  P 0  in  P, we finally obtain a linkage that contradicts the i

j

choice of  P. This completes the proof of (1).

Vy ∈ V  r ( U ∪ W)

V ∈ W

x

wy

P 0i

Pi

Pi

w

P 0j

Pj

P

H

P 0

W ∈ W

Fig. 8.1.3.  If  Pi  ends in  Vx, we replace  Pi, Pj, P  by  P 0i, P 0j, P 0

With the help of (1) we may define an injection  W → U  as follows: given  W ∈ W, choose a link that passes through  W  and next meets a set  U ∈ U, and map  W 7→ U . (This is indeed an injection, because different links end in different branch sets.) Thus  |U| >  |W|, and hence

|U| >  dk/ 2 e.

Let us assume the enumeration of  v 1 , . . . , vk  to be such that the first  u :=  |U|  of the links  P 1 , . . . , Pk  end in sets from  U. Since 2 δ( H) > u

174

8. Substructures in Sparse Graphs

|H| + 3 k, we can find for any two sets  V

k  sets  V

2

x, Vy ∈ U  at least 3

2

z  such

that  xz, yz ∈ E( H). At least  k/ 2 of these sets  Vz  do not lie in  U ∪ W.

Thus whenever  U 1 , . . . , U 2 h  are distinct sets in  U (so  h  6  u/ 2 6  k/ 2), we may find inductively  h  distinct sets  V i ∈ V  r ( U ∪ W) ( i = 1 , . . . , h) such that  V i  is joined in  G  to both  U 2 i− 1 and  U 2 i. For each  i, any vertex of U 2 i− 1 can be linked by a path through  V i  to any desired vertex of  U 2 i, and these paths will be disjoint for different  i. Joining up the appropriate pairs of paths from  P  in this way, we see that the set  { v 1 , . . . , vu }  is linked in  G, and the lemma is proved.

¤

Lemma 8.1.3.  Let k > 6  be an integer. Then every graph G with ε( G) >  k has a minor H such that  2 δ( H) >  |H| + 1  k.

6

G 0

Proof .

We begin by choosing a (4-)minimal minor  G 0 of  G  with ε( G 0) >  k. The minimality of  G 0 implies that  δ( G 0)  > k  and  ε( G 0) =  k (otherwise we could delete a vertex or an edge of  G 0), and hence k + 1 6  δ( G 0) 6  d( G 0) = 2 k .

x 0

Let  x 0  ∈ G 0 be a vertex of minimum degree.

If  k  is odd, let  m := ( k + 1) / 2 and

G 1 :=  G 0 [  { x 0  } ∪ NG ( x

0

0) ]  .

Then  |G 1 | =  δ( G 0) + 1 6 2 k + 1 6 2( k + 1) = 4 m. By the minimality of  G 0, contracting any edge  x 0 y  of  G 0 will result in the loss of at least  k + 1 edges. The vertices  x 0 and  y  thus have at least  k  common neighbours, so  δ( G 1) >  k + 1 = 2 m (Fig. 8.1.4).

y (

>  k

x 0

NG ( x

0

0)

Fig. 8.1.4.  The graph  G 1 4  G: a first approximation to the desired minor  H

If  k  is even, we let  m :=  k/ 2 and

G 1 :=  G 0 [  NG ( x

0

0) ]  .

Then  |G 1 | =  δ( G 0) 6 2 k = 4 m, and  δ( G 1) >  k = 2 m  as before.

8.1 Topological minors

175

Thus in either case we have found an integer  m >  k/ 2 and a graph m

G 1 4  G  such that

G 1

|G 1 |  6 4 m

(1)

and  δ( G 1) > 2 m, so

ε( G 1) >  m >  k/ 2 > 3  .

(2)

As 2 δ( G 1) > 4 m >  |G 1 |, our graph  G 1 is already quite a good candidate for the desired minor  H  of  G. In order to jack up its value of 2 δ  by another 1  k (as required for  H), we shall reapply the above 6

contraction process to  G 1, and a little more rigorously than before: step by step, we shall contract edges as long as this results in a loss of no more than 7  m  edges per vertex. In other words, we permit a loss of edges 6

slightly greater than maintaining  ε >  m  seems to allow. (Recall that, when we contracted  G  to  G 0, we put this threshold at  ε( G) =  k.)  If  this second contraction process terminates with a non-empty graph  H 0, then ε( H 0) will be at least 7  m, higher than for  G

m  thus gained will

6

1! The 1

6

suffice to give the graph  H 1, obtained from  H 0 just as  G 1 was obtained from  G 0, the desired high minimum degree.

But how can we be sure that this second contraction process will

indeed end with a non-empty graph? Paradoxical though it may seem,

the reason is that even a permitted loss of up to 7  m  edges (and one 6

vertex) per contraction step cannot destroy the  m |G 1 |  or more edges of  G 1 in the  |G 1 |  steps possible: the graphs with fewer than  m  vertices towards the end of the process would simply be too small to be able to

shed their allowance of 7  m  edges—and, by (1), these small graphs would 6

account for about a quarter of the process!

Formally, we shall control the graphs  H  in the contraction process not by specifying an upper bound on the number of edges to be discarded at each step, but by fixing a lower bound for  kHk  in terms of  |H|. This

¡ ¢

bound grows linearly from a value of just above  m  for  |H| =  m  to a 2

value of less than 4 m 2 for  |H| = 4 m. By (1) and (2),  H =  G 1 will satisfy this bound, but clearly it cannot be satisfied by any  H  with  |H| =  m; so the contraction process must stop somewhere earlier with  |H| > m.

To implement this approach, let

f ( n) := 1  m( n − m −  5)

f

6

and

n

¡ ¢o

H :=  H  4  G

m

1 :  kH k >  m |H | +  f ( |H |)  −

.

2

H

By (1),

¡ ¢

f ( |G

m

1 |) 6  f (4 m) = 1  m 2  −  5  m <

,

2

6

2

so  G 1  ∈ H  by (2).

176

8. Substructures in Sparse Graphs

For every  H ∈ H, any graph obtained from  H  by one of the following three operations will again be in  H:

¡ ¢

(i) deletion of an edge, if  kHk >  m |H| +  f ( |H|)  − m + 1; 2

(ii) deletion of a vertex of degree at most 7  m;

6

(iii) contraction of an edge  xy ∈ H  such that  x  and  y  have at most 7  m −  1 common neighbours in  H.

6

Starting with  G 1, let us apply these operations as often as possible, and H 0

let  H 0  ∈ H  be the graph obtained eventually. Since

¡ ¢

kKmk =  m |Km| − m − m 2

and

f ( m) =  −  5  m > −m ,

6

Km  does not have enough edges to be in  H; thus,  H  contains no graph x 1

on  m  vertices. Hence  |H 0 | > m, and in particular  H 0  6=  ∅. Let  x 1  ∈ H 0

be a vertex of minimum degree, and put

H 1

H 1 :=  H 0 [  { x 1  } ∪ NH ( x

0

1) ]  .

We shall prove that the minimum degree of  H :=  H 1 is as large as claimed in the lemma.

Note first that

δ( H 1)  >  7  m .

(3)

6

Indeed, since  H 0 is minimal with respect to (ii) and (iii), we have  d( x 1)  > 7  m  in  H

6

0 (and hence in  H 1), and every vertex  y 6=  x 1 of  H 1 has more than 7  m −  1 common neighbours with  x

m

6

1 (and hence more than 7

6

neighbours in  H 1 altogether). In order to convert (3) into the desired inequality of the form

2 δ( H 1) >  |H 1 | +  αm ,

we need an upper bound for  |H 1 |  in terms of  m. Since  H 0 lies in  H  but is minimal with respect to (i), we have

³

´ ¡ ¢

kH

1

m

0 k < m |H 0 | +

m |H

m 2  −  5  m −

+ 1

6

0 | −  1

6

6

2

= 7  m |H

m 2  −  1  m + 1

6

0 | −  4

6

3

6 7 m |H

m 2 .

(4)

6

0 | −  4

6

(2)

8.1 Topological minors

177

By the choice of  x 1 and definition of  H 1, therefore,

|H 1 | −  1 =  δ( H 0)

6 2  ε( H 0)

<  7  m −  4  m 2 /|H

3

3

0 |

(4)

6 7 m −  1 m

3

3

(1)

= 2 m ,

so  |H 1 |  6 2 m. Hence,

2 δ( H 1)  >  2 m + 1  m

3

(3)

>  |H 1 | + 1 m

3

>  |H 1 | + 1 k

6

(2)

as claimed.

¤

Proof of Theorem 8.1.1. We prove the assertion for  c := 1116. Let

(1.4.2)

G  be a graph with  d( G) > 1116 r 2. By Theorem 1.4.2,  G  has a subgraph G 0 such that

G 0

κ( G 0) > 279 r 2 > 276 r 2 + 3 r .

Pick a set  X :=  { x 1 , . . . , x 3 r }  of 3 r  vertices in  G 0, and let  G 1 :=  G 0  − X.

X

For each  i = 1 , . . . ,  3 r  choose a set  Yi  of 5 r  neighbours of  xi  in  G 1; let G 1 , Yi

these sets  Yi  be disjoint for different  i. (This is possible since  δ( G 0) > κ( G 0) > 15 r 2 +  |X|.)

As

δ( G 1) >  κ( G 1) >  κ( G 0)  − |X| > 276 r 2 , we have  ε( G 1) > 138 r 2. By Lemma 8.1.3,  G 1 has a minor  H  with 2 δ( H) >  |H| + 23 r 2 and is therefore (15 r 2 ,  7 r 2)-linked by Lemma 8.1.2;

S

let  Z ⊆

3 r

Y

i=1

i  be a set of 7 r 2 vertices that is linked in  G 1.

Z

For all  i = 1 , . . . ,  3 r  let  Zi :=  Z ∩ Yi. Since  Z  is linked, it suffices Zi

to find  r  indices  i  with  |Zi| >  r −  1: then the corresponding  xi  will be the branch vertices of a  T Kr  in  G 0. If  r  such  i  cannot be found, then

|Zi|  6  r −  2 for all but at most  r −  1 indices  i. But then 3 r

X

|Z| =

|Zi|  6 ( r −  1) 5 r + (2 r + 1)( r −  2)  <  7 r 2 =  |Z| , i=1

a contradiction.

¤

178

8. Substructures in Sparse Graphs

Although Theorem 8.1.1 already gives a good estimate, it seems very difficult to determine the exact average degree needed to force a

T Kr  subgraph, even for small  r. We shall come back to the case of r = 5 in Section 8.3; more results and conjectures are given in the notes.

The following almost counter-intuitive result of Mader implies that

the existence of a topological  Kr  minor can be forced essentially by large girth. In the next section, we shall prove the analogue of this for ordinary minors.

Theorem 8.1.4. (Mader 1997)

For every graph H of maximum degree d > 3  there exists an integer k such that every graph G of minimum degree at least d and girth at least k contains H as a topological minor.

As discussed already in Chapter 5.2 and the introduction to Chap-

ter 7, no constant average degree, however large, will force an arbitrary graph to contain a given graph  H  as a subgraph—as long as  H  contains at least one cycle. By Proposition 1.2.2 and Corollary 1.5.4, on the other hand, any graph  G  contains all trees on up to  ε( G) + 2 vertices. Large average degree therefore does ensure the occurrence of any fixed tree  T

as a subgraph. What can we say, however, if we would like  T  to occur as an  induced  subgraph?

Here, a large average degree appears to do as much harm as good,

even for graphs of bounded clique number.

(Consider, for example,

complete bipartite graphs.) It is all the more remarkable, then, that

the assumption of a large chromatic number rather than a large average

degree seems to make a difference here: according to a conjecture of

Gyárfás, any graph of large enough chromatic number contains either a

large complete graph or any given tree as an induced subgraph. (For-

mally: for every integer  r  and every tree  T , there exists an integer  k  such that every graph  G  with  χ( G) >  k  and  ω( G)  < r  contains an induced copy of  T .)

The weaker topological version of this is indeed true:

Theorem 8.1.5. (Scott 1997)

For every integer r and every tree T there exists an integer k such that every graph with χ( G) >  k and ω( G)  < r contains an induced copy of some subdivision of T .

8.2 Minors

179

8.2 Minors

According to Theorem 8.1.1, an average degree of  cr 2 suffices to force the existence of a topological  Kr  minor in a given graph. If we are content with any minor, topological or not, an even smaller average degree will do: in a pioneering paper of 1968, Mader proved that every graph with an average degree of at least  cr  log  r  has a  Kr  minor. The following result, the analogue to Theorems 7.1.1 and 8.1.1 for general minors, determines the precise average degree needed as a function of  r, up to a constant  c: Theorem 8.2.1. (Kostochka 1982; Thomason 1984)

There exists a c ∈  R  such that, for every r ∈  N , every graph G of average

√

degree d( G) >  c r  log  r has a Kr minor. Up to the value of c, this bound is best possible as a function of r.

The easier implication of the theorem, the fact that in general an average

√

degree of  c r  log  r  is needed to force a  Kr  minor, follows from considering random graphs, to be introduced in Chapter 11. The converse impli-

cation, the fact that this average degree suffices, is proved by methods similar to those described in Section 8.1.

Rather than proving Theorem 8.2.1, we therefore devote the remain-

der of this section to another striking result on forcing minors. At first glance, this result is so surprising that it seems almost paradoxical: as long as we do not merely subdivide edges, we can force a  Kr  minor in a graph simply by raising its girth (Corollary 8.2.3)!

Theorem 8.2.2. (Thomassen 1983)

Given an integer k, every graph G with girth g( G) > 4 k −  3  and δ( G) > 3

has a minor H with δ( H) >  k.

Proof . As  δ( G) > 3, every component of  G  contains a cycle. In particu-

(1.5.3)

lar, the assertion is trivial for  k  6 2; so let  k > 3. Consider the vertex set  V  of a component of  G, together with a partition  { V 1 , . . . , Vm }  of V, Vi

V  into as many connected sets  Vi  with at least 2 k −  2 vertices each as m

possible. (Such a partition exists, since  |V | >  g( G)  >  2 k −  2 and  V  is connected in  G.)

We first show that every  G [  Vi ] is a tree. To this end, let  Ti  be a spanning tree of  G [  Vi ]. If  G [  Vi ] has an edge  e /

∈ Ti, then  Ti +  e  contains

a cycle  C; by assumption,  C  has length at least 4 k −  3. The edge (about) opposite  e  on  C  therefore separates the path  C − e, and hence also  Ti, into two components with at least 2 k −  2 vertices each. Together with the sets  Vj  for  j 6=  i, these two components form a partition of  V  into m + 1 sets that contradicts the maximality of  m.

So each  G [  Vi ] is indeed a tree, i.e.  G [  Vi ] =  Ti. As  δ( G) > 3, the Ti

degrees in  G  of the vertices in  Vi  sum to at least 3  |Vi|, while the edges of  Ti  account for only 2  |Vi| −  2 in this sum. Hence for each  i,  G  has

180

8. Substructures in Sparse Graphs

at least  |Vi| + 2 > 2 k  edges joining  Vi  to  V  r  Vi. We shall prove that every  Vi  sends at most two edges to each of the other  Vj; then  Vi  must send edges to at least  k  of those  Vj, so the  Vi  are the branch sets of an M H ⊆ G  with  δ( H) >  k.

Suppose, without loss of generality, that  G  has three  V 1– V 2 edges.

Then there are vertices  v 1  ∈ V 1 and  v 2  ∈ V 2 such that  G [  V 1  ∪ V 2 ] contains three independent  v 1– v 2 paths  P 1,  P 2,  P 3 (Fig. 8.2.1). At most one of P 1

P 0 1

P 0

P 2

v

2

1

v 2

P 0

V

3

1

P

V

3

2

Fig. 8.2.1.  Three edges between  V 1 and  V 2

these paths can be shorter than 1  g( G). We assume that  P

2

1 has length

at least  d  1  g( G) e > 2 k −  1 and let  P 0

| > 2 k −  2. Since

2

1 := ˚

P 1; then  |P 0 1

P 2  ∪ P 3 is a cycle of length at least 4 k −  3, we can further find disjoint paths  P 0

⊆

2 , P 0

3

P 2  ∪ P 3 with 2 k −  2 vertices each. Since  G [  V 1  ∪ V 2 ] is connected, there exists a partition of  V 1  ∪ V 2 into three connected sets V 0 1 , V 0 2 , V 0 3 such that  V ( P 0)  ⊆ V 0  for  i = 1 ,  2 ,  3. Replacing the two sets i

i

V 1 , V 2 in our partition of  V  with the three sets  V 0 1 , V 0 2 , V 0 3, we obtain a partition of  V  that contradicts the maximality of  m.

¤

The following combination of Theorems 8.2.1 and 8.2.2 brings out the paradoxical character of the latter particularly well:

Corollary 8.2.3.  There exists a c ∈  R  such that, for every r ∈  N , every

√

graph G with girth g( G) >  c r  log  r and δ( G) > 3  has a Kr minor.

Proof . We prove the corollary for  c := 4 c0, where  c0  is the constant from

Theorem 8.2.1. Let  G  be given as stated. By Theorem 8.2.2,  G  has a

√

minor  H  with  δ( H) >  c0r  log  r. By Theorem 8.2.1,  H (and hence  G) has a  Kr  minor.

¤

8.3 Hadwiger’s conjecture

181

8.3 Hadwiger’s conjecture

√

As we saw in the preceding two sections, an average degree of  c r  log  r suffices to force an arbitrary graph to have a  Kr  minor, and an average degree of  cr 2 forces it to have a topological  Kr  minor. If we replace

‘average degree’ above with ‘chromatic number’ then, with almost the

same constants  c, the two assertions remain true: this is because every graph with chromatic number  k  has a subgraph of average degree at least  k −  1 (Corollary 5.2.3).

√

Although both functions above,  c r  log  r  and  cr 2, are best possible (up to the constant  c) for the said implications with ‘average degree’, the question arises whether they are still best possible with ‘chromatic number’—or whether some slower-growing function would do in that

case. What is lurking behind this problem about growth rates, of course, is a fundamental question about the nature of the invariant  χ: can this invariant have some direct  structural  effect on a graph in terms of forcing concrete substructures, or is its effect no greater than that of the ‘un-structural’ property of having lots of edges somewhere, which it implies trivially?

Neither for general nor for topological minors is the answer to this

question known. For general minors, however, the following conjecture

of Hadwiger suggests a positive answer; the conjecture is considered by many as one of the deepest open problems in graph theory.

Conjecture. (Hadwiger 1943)

The following implication holds for every integer r >  0  and every graph G:

χ( G) >  r ⇒ G <  Kr.

Hadwiger’s conjecture is trivial for  r  6 2, easy for  r = 3 and  r = 4

(exercises), and equivalent to the four colour theorem for  r = 5 and r = 6. For  r > 7, the conjecture is open. Rephrased as  G <  Kχ( G), it is true for almost all graphs.3 In general, the conjecture for  r + 1 implies it for  r (exercise).

The Hadwiger conjecture for any fixed  r  is equivalent to the assertion that every graph without a  Kr  minor has an ( r −  1)-colouring. In this reformulation, the conjecture raises the question of what the graphs without a  Kr  minor look like: any sufficiently detailed structural description of those graphs should enable us to decide whether or not they can be ( r −  1)-coloured.

For  r = 3, for example, the graphs without a  Kr  minor are precisely the forests (why?), and these are indeed 2-colourable. For  r = 4, there 3 See Chapter 11 for the notion of ‘almost all’.

182

8. Substructures in Sparse Graphs

is also a simple structural characterization of the graphs without a  Kr minor:

[ 12.4.2 ]

Proposition 8.3.1.  A graph with at least three vertices is edge-maximal without a K 4  minor if and only if it can be constructed recursively from triangles by pasting 4  along K 2 s.

(1.7.2)

Proof . Recall first that every  M K 4 contains a  T K 4, because ∆( K 4) = 3

(4.4.4)

(Proposition 1.7.2); the graphs without a  K 4 minor thus coincide with those without a topological  K 4 minor. The proof that any graph constructible as described is edge-maximal without a  K 4 minor is left as an easy exercise; in order to deduce Hadwiger’s conjecture for  r = 4, we only need the converse implication anyhow. We prove this by induction

on  |G|.

Let  G  be given, edge-maximal without a  K 4 minor. If  |G| = 3 then G  is itself a triangle, so let  |G| > 4 for the induction step. Then  G  is not complete; let  S ⊆ V ( G) be a separating set with  |S| =  κ( G), and let C 1 , C 2 be distinct components of  G − S. Since  S  is a minimal separator, every vertex in  S  has a neighbour in  C 1 and another in  C 2. If  |S| > 3, this implies that  G  contains three independent paths  P 1 , P 2 , P 3 between a vertex  v 1  ∈ C 1 and a vertex  v 2  ∈ C 2. Since  κ( G) =  |S| > 3, the graph G − { v 1 , v 2  }  is connected and contains a (shortest) path  P  between two different  Pi. Then  P ∪ P 1  ∪ P 2  ∪ P 3 =  T K 4, a contradiction.

Hence  κ( G) 6 2, and the assertion follows from Lemma 4.4.45 and the induction hypothesis.

¤

One of the interesting consequences of Proposition 8.3.1 is that all the edge-maximal graphs without a  K 4 minor have the same number of edges, and are thus all ‘extremal’:

Corollary 8.3.2.  Every edge-maximal graph G without a K 4  minor has  2  |G| −  3  edges.

Proof . Induction on  |G|.

¤

Corollary 8.3.3.  Hadwiger’s conjecture  holds for r = 4 .

Proof . If  G  arises from  G 1 and  G 2 by pasting along a complete graph, then  χ( G) = max  { χ( G 1) , χ( G 2)  } (see the proof of Proposition 5.5.2).

Hence, Proposition 8.3.1 implies by induction on  |G|  that all edge-maximal (and hence all) graphs without a  K 4 minor can be 3-coloured.

¤

4 This was defined formally in Chapter 5.5.

5 The proof of this lemma is elementary and can be read independently of the rest of Chapter 4.

8.3 Hadwiger’s conjecture

183

It is also possible to prove Corollary 8.3.3 by a simple direct argument

(Exercise 13).

By the four colour theorem, Hadwiger’s conjecture for  r = 5 follows from the following structure theorem for the graphs without a  K 5 minor, just as it follows from Proposition 8.3.1 for  r = 4. The proof of Theorem 8.3.4 is similar to that of Proposition 8.3.1, but considerably longer. We therefore state the theorem without proof:

Theorem 8.3.4. (Wagner 1937)

Let G be an edge-maximal graph without a K 5  minor. If |G| > 4  then G can be constructed recursively, by pasting along triangles and K 2 s, from plane triangulations and copies of the graph W (Fig. 8.3.1).

=

=

Fig. 8.3.1.  Three representations of the  Wagner graph W

Using Corollary 4.2.8, one can easily compute which of the graphs

4.2.8

constructed as in Theorem 8.3.4 have the most edges. It turns out that

these  extremal  graphs without a  K 5 minor have no more edges than those that are extremal with respect to  { M K 5 , M K 3 ,  3  }, i.e. the maximal planar graphs:

Corollary 8.3.5.  A graph with n vertices and no K 5  minor has at most 3 n −  6  edges.

¤

Since  χ( W ) = 3, Theorem 8.3.4 and the four colour theorem imply

Hadwiger’s conjecture for  r = 5:

Corollary 8.3.6.  Hadwiger’s conjecture  holds for r = 5 .

¤

The Hadwiger conjecture for  r = 6 is again substantially more difficult than the case  r = 5, and again it relies on the four colour theorem. The proof shows (without using the four colour theorem) that any minimal-order counterexample arises from a planar graph by adding one

vertex—so by the four colour theorem it is not a counterexample after all.

Theorem 8.3.7. (Robertson, Seymour & Thomas 1993)

Hadwiger’s conjecture  holds for r = 6 .

184

8. Substructures in Sparse Graphs

By Corollary 8.3.5, any graph with  n  vertices and more than 3 n −  6

edges contains an  M K 5. In fact, it even contains a  T K 5. This inconspicuous improvement is another deep result that had been conjectured

for over 30 years:

Theorem 8.3.8. (Mader 1998)

Every graph with n vertices and more than  3 n −  6  edges contains K 5  as a topological minor.

No structure theorem for the graphs without a  T K 5, analogous to

Proposition 8.3.1 and Theorem 8.3.4, is known. However, Mader has characterized those with the greatest possible number of edges:

Theorem 8.3.9. (Mader 1997)

A graph is extremal without a T K 5  if and only if it can be constructed recursively from maximal planar graphs by pasting along triangles.

Exercises

1.

Prove, from first principles, the theorem of Wagner (1964) that every

graph of chromatic number at least 2 r  contains  Kr  as a minor.

(Hint. Apply induction on r.)

2.

Prove, from first principles, the result of Mader (1967) that every graph of average degree at least 2 r− 2 contains  Kr  as a minor.

(Hint. Induction on  r.)

3.  −  Derive Wagner’s theorem (Ex. 1) from Mader’s theorem (Ex. 2).

4.+ Given an integer  r >  0, find an integer  k  such that every grid with  k additional edges has a  Kr  minor, provided that all the ends of the new edges have distance at least  k  in the grid both from each other and from the grid boundary. ( Grids  are defined in Chapter 12.3.)

5.+ Show that any function  h  as in Theorem 3.6.1 satisfies the inequality h( r)  >  1  r 2 for all even  r, and hence that Theorem 8.1.1 is best possible 8

up to the value of the constant  c.

6.

Prove the statement of Lemma 8.1.3 for  k <  6.

7.

Explain how exactly the term of 1  k  in the statement of Lemma 8.1.3

6

is used in the proof of Theorem 8.1.1. Could it be replaced by  k/ 1000, or by zero?

8.

Explain how exactly the number 7 in the proof of Lemma 8.1.3 was 6

arrived at. Could it be replaced by 3 ?

2

Exercises

185

9.+ For which trees  T  is there a function  f : N  →  N tending to infinity, such that every graph  G  with  χ( G)  < f ( d( G)) contains an induced copy of  T ?

(In other words: can we force the chromatic number up by raising the

average degree, as long as  T  does not occur as an induced subgraph?

Or, as in Gy´

arf´

as’s conjecture: will a large average degree force an induced copy of  T  if the chromatic number is kept small?)

10.  −  Derive the four colour theorem from Hadwiger’s conjecture for  r = 5.

11.  −  Show that Hadwiger’s conjecture for  r + 1 implies the conjecture for  r.

12.  −  Using the results from this chapter, prove the following weakening of

Hadwiger’s conjecture: given any  ² >  0, every graph of chromatic number at least  r 1+ ²  has a  Kr  minor, provided that  r  is large enough.

13.+ Prove Hadwiger’s conjecture for  r = 4 from first principles.

14.+ Prove Hadwiger’s conjecture for line graphs.

15.

(i) −  Show that Hadwiger’s conjecture is equivalent to the statement that  G <  Kχ( G) for all graphs  G.

(ii) Show that any minimum-order counterexample  G  to Hadwiger’s

conjecture (as rephrased above) satisfies  Kχ( G) − 1  6⊆ G  and has a connected complement.

16.

Show that any graph constructed as in Theorem 8.3.1 is edge-maximal without a  K 4 minor.

17.

Prove the implication  δ( G) > 3  ⇒ G ⊇ T K 4.

(Hint. Theorem 8.3.1.)

18.

A multigraph is called  series-parallel  if it can be constructed recursively from a  K 2 by the operations of subdividing and of doubling edges. Show that a 2-connected multigraph is series-parallel if and only if it has no (topological)  K 4 minor.

19.

Prove Corollary 8.3.5.

20.

Characterize the graphs with  n  vertices and more than 3 n −  6 edges that contain no  T K 3 ,  3. In particular, determine ex( n, T K 3 ,  3).

(Hint. By a theorem of Wagner, every edge-maximal graph without a

K 3 ,  3 minor can be constructed recursively from maximal planar graphs and copies of  K 5 by pasting along  K 2s.)

21.

By a theorem of Pelikán, every graph of minimum degree at least 4

contains a subdivision of  K 5

−, a  K 5 minus an edge. Using this theorem,

prove Thomassen’s 1974 result that every graph with  n > 5 vertices and at least 4 n −  10 edges contains a  T K 5.

(Hint. Show by induction on  |G|  that if  kGk > 4 n −  10 then for every vertex  x ∈ G  there is a  T K 5  ⊆ G  in which  x  is not a branch vertex.)

186

8. Substructures in Sparse Graphs

Notes

The investigation of graphs not containing a given graph as a minor, or topological minor, has a long history. It probably started with Wagner’s 1935

PhD thesis, in which he sought to ‘detopologize’ the four colour problem by classifying the graphs without a  K 5 minor. His hope was to be able to show abstractly that all those graphs were 4-colourable; since the graphs without a  K 5 minor include the planar graphs, this would amount to a proof of the four colour conjecture involving no topology whatsoever. The result of Wagner’s efforts, Theorem 8.3.4, falls tantalizingly short of this goal: although it succeeds in classifying the graphs without a  K 5 minor in structural terms, planarity re-emerges as one of the criteria used in the classification. From this point of view, it is instructive to compare Wagner’s  K 5 theorem with similar classification theorems, such as his analogue for  K 4 (Proposition 8.3.1), where the graphs are decomposed into parts from a  finite  set of irreducible graphs.

See R. Diestel,  Graph Decompositions, Oxford University Press 1990, for more such classification theorems.

Despite its failure to resolve the four colour problem, Wagner’s  K 5 struc-

ture theorem had consequences for the development of graph theory like few others. To mention just two: it prompted Hadwiger to make his famous conjec-

ture; and it inspired the notion of a tree-decomposition, which is fundamental to the work of Robertson and Seymour on minors (see Chapter 12). Wagner himself responded to Hadwiger’s conjecture with a proof that, in order to force a  Kr  minor, it does suffice to raise the chromatic number of a graph to  some value depending only on  r (Exercise 1). This theorem then, along with its analogue for topological minors proved independently by Dirac and by Jung, prompted the question of which average degree suffices to force the desired minor.

The deepest contribution in this field of research was no doubt made

by Mader, in a series of papers from the late sixties. Our proof of Lemma

8.1.3 is presented intentionally in a step-by-step fashion, to bring out some of Mader’s ideas. Mader’s own proof—not to mention that of Thomason’s best possible version of the lemma, as used in the original proof of Theorem 8.1.1—

is wrapped up so elegantly that it becomes hard to see the ideas behind it.

Except for this lemma, our proof of Theorem 8.1.1 follows B. Bollobás & A.G. Thomason, Proof of a conjecture of Mader, Erd˝

os and Hajnal on to-

pological complete subgraphs,  Europ. J. Combinatorics 19 (1998), 883–887.

The constant  c  from the theorem was shown by J. Komlós & E. Szemerédi, Topological cliques in graphs II,  Combinatorics, Probability and Computing 5

(1996), 79–90, to be no greater than about 1 , which is not far from the lower 2

bound of 1 given in Exercise 5.

8

Theorem 8.1.4 is from W. Mader, Topological subgraphs in graphs of large girth,  Combinatorica 18 (1998), 405–412. For  H =  Kr, the theorem says that every graph  G  with  δ( G) >  r −  1 and  g( G) large contains a  T Kr. For  r = 5, Mader conjectured that  g( G) > 5 should be enough, and that the requirement of  δ( G) > 4 could be weakened further: he conjectured that any graph of girth at least 5, large enough order  n, and 2 n −  4 or more edges has a topological  K 5

minor. (To see that this implies the minimum degree version of the conjecture even for small order, consider enough disjoint copies of the given graph.) For

Notes

187

general  H, Mader improved Theorem 8.1.4 by weakening the requirement of δ( G) >  d  to  d( G) >  d −  1 +  ²  for arbitrary  ² >  0 (where now the girth  k  required to force a  T H  in such graphs  G  depends on  ²  as well as on  H); see W. Mader, Subdivisions of a graph of maximal degree  n + 1 in graphs of average degree n +  ²  and large girth, manuscript 1999.

Theorem 8.1.5 is due to A.D. Scott, Induced trees in graphs of large chromatic number,  J. Graph Theory 24 (1997), 297–311.

Theorem 8.2.1 was

proved independently by Kostochka (1982; English translation: A.V. Kostochka, Lower bounds of the Hadwiger number of graphs by their average degree, Combinatorica 4 (1984), 307–316) and by A.G. Thomason, An extremal function for contractions of graphs,  Math. Proc. Camb. Phil. Soc. 95 (1984), 261–

265. Theorem 8.2.2 was taken from Thomassen’s survey, Paths, Circuits and Subdivisions, in (L.W. Beineke & R.J. Wilson, eds.)  Selected Topics in Graph Theory 3, Academic Press 1988.

The proof of Hadwiger’s conjecture for  r = 4, hinted at in Exercise 13, is given by Hadwiger himself in the 1943 paper containing his conjecture. For a while, there was a counterpart to Hadwiger’s conjecture for topological minors, the conjecture of Hajós that  χ( G) >  r  even implies  G ⊇ T Kr. A counterexample to this conjecture was found in 1979 by Catlin; a little later, Erd˝

os and

Fajtlowicz even proved that Hajós’s conjecture is false for  almost all  graphs (see Chapter 11).

Mader’s Theorem 8.3.8 that 3 n −  5 edges force a topological  K 5 minor had been conjectured by Dirac in 1964. Its proof comprises two papers: W. Mader, 3 n −  5 edges do force a subdivision of  K 5,  Combinatorica 18 (1998), 569–595; and W. Mader, An extremal problem for subdivisions of  K−,  J. Graph Theory 5

30 (1999), 261–276. His proof of Theorem 8.3.9 has not been published yet.

Dirac’s conjecture has been extended by Seymour, who conjectures that every 5-connected non-planar graph should contain a  T K 5 (unpublished).

9

Ramsey Theory

for Graphs

In this chapter we set out from a type of problem which, on the face of it, appears to be similar to the theme of the last two chapters: what kind of substructures are necessarily present in every large enough graph?

The regularity lemma of Chapter 7.2 provides one possible answer to this question, saying as it does that every (large) graph  G  contains large random-like bipartite subgraphs. If we are looking for more defi-nite substructures, however, such as subgraphs isomorphic to some given graphs  H, then these  H  will have to be sufficiently complementary in kind to cater for the variety allowed for  G. For example: given an integer  r, does every large enough graph contain either a  Kr  or an induced  Kr? Does every large enough connected graph contain either a Kr  or else a large induced path or star?

Despite its similarity to extremal problems in that we are looking

for local implications of global assumptions, the above type of question leads to a kind of mathematics with a distinctive flavour of its own.

Indeed, the theorems and proofs in this chapter have more in common

with similar results in algebra or geometry, say, than with most other

areas of graph theory. The study of their underlying methods, therefore, is generally regarded as a combinatorial subject in its own right: the

discipline of  Ramsey theory.

In line with the subject of this book, we shall focus on results that

are naturally expressed in terms of graphs. Even from the viewpoint of

general Ramsey theory, however, this is not as much of a limitation as

it might seem: graphs are a natural setting for Ramsey problems, and

the material in this chapter brings out a sufficient variety of ideas and methods to convey some of the fascination of the theory as a whole.

190

9. Ramsey Theory

9.1 Ramsey’s original theorems

In its simplest version, Ramsey’s theorem says that, given an integer

r > 0, every large enough graph  G  contains either  Kr  or  Kr  as an induced subgraph. At first glance, this may seem surprising: after all, we need about ( r −  2) /( r −  1) of all possible edges to force a  Kr  subgraph in  G

(Cor. 7.1.3), but neither  G  nor  G  can be expected to have more than half the total number of edges. However, as the Turán graphs illustrate well, squeezing many edges into  G  without creating a  Kr  imposes additional structure on  G, which may help us find an induced  Kr.

So how could we go about proving Ramsey’s theorem? Let us try

to build a  Kr  or  Kr  in  G  inductively, starting with an arbitrary vertex v 1  ∈ V 1 :=  V ( G). If  |G|  is large, there will be a large set  V 2  ⊆ V 1 r  { v 1  }

of vertices that are either all adjacent to  v 1 or all non-adjacent to  v 1.

Accordingly, we may think of  v 1 as the first vertex of a  Kr  or  Kr  whose other vertices all lie in  V 2. Let us then choose another vertex  v 2  ∈ V 2

for our  Kr  or  Kr. Since  V 2 is large, it will have a subset  V 3, still fairly large, of vertices that are all ‘of the same type’ with respect to  v 2 as well: either all adjacent or all non-adjacent to it. We then continue our search for vertices inside  V 3, and so on (Fig. 9.1.1).

v 1

V

v 1

2

V 3

v 2

Fig. 9.1.1.  Choosing the sequence  v 1 , v 2 , . . .

How long can we go on in this way? This depends on the size of

our initial set  V 1: each set  Vi  has at least half the size of its predeces-sor  Vi− 1, so we shall be able to complete  s  construction steps if  G  has order about 2 s. As the following proof shows, the choice of  s = 2 r −  3

vertices  vi  suffices in order to find among them the vertices of a  Kr or  Kr.

Theorem 9.1.1. (Ramsey 1930)

[ 9.2.2 ]

For every r ∈  N  there exists an n ∈  N  such that every graph of order at least n contains either Kr or Kr as an induced subgraph.

Proof . The assertion is trivial for  r  6 1; we assume that  r > 2. Let n := 22 r− 3, and let  G  be a graph of order at least  n. We shall define a sequence  V 1 , . . . , V 2 r− 2 of sets and choose vertices  vi ∈ Vi  with the following properties:

(i)  |Vi| = 22 r− 2 −i ( i = 1 , . . . ,  2 r −  2);

9.1 Ramsey’s original theorems

191

(ii)  Vi ⊆ Vi− 1 r  { vi− 1  } ( i = 2 , . . . ,  2 r −  2); (iii)  vi− 1 is adjacent either to all vertices in  Vi  or to no vertex in  Vi ( i = 2 , . . . ,  2 r −  2).

Let  V 1  ⊆ V ( G) be any set of 22 r− 3 vertices, and pick  v 1  ∈ V 1 arbitrarily.

Then (i) holds for  i = 1, while (ii) and (iii) hold trivially. Suppose now that  Vi− 1 and  vi− 1  ∈ Vi− 1 have been chosen so as to satisfy (i)–(iii) for i −  1, where 1  < i  6 2 r −  2. Since

|Vi− 1 r  { vi− 1  }| = 22 r− 1 −i −  1

is odd,  Vi− 1 has a subset  Vi  satisfying (i)–(iii); we pick  vi ∈ Vi  arbitrarily.

Among the 2 r −  3 vertices  v 1 , . . . , v 2 r− 3, there are  r −  1 vertices that show the same behaviour when viewed as  vi− 1 in (iii), being adjacent either to all the vertices in  Vi  or to none. Accordingly, these  r −  1 vertices and  v 2 r− 2 induce either a  Kr  or a  Kr  in  G, because  vi, . . . , v 2 r− 2  ∈ Vi for all  i.

¤

The least integer  n  associated with  r  as in Theorem 9.1.1 is the  Ramsey Ramsey

number R( r) of  r; our proof shows that  R( r) 6 22 r− 3. In Chapter 11 we number

R( r)

shall use a simple probabilistic argument to show that  R( r) is bounded below by 2 r/ 2 (Theorem 11.1.3).

It is customary in Ramsey theory to think of partitions as colourings:

a  colouring  of (the elements of) a set  X with c colours, or  c-colouring  for c-colouring

short, is simply a partition of  X  into  c  classes (indexed by the ‘colours’).

In particular, these colourings need not satisfy any non-adjacency re-

quirements as in Chapter 5. Given a  c-colouring of [ X] k, the set of all

[ X] k

k-subsets of  X, we call a set  Y ⊆ X monochromatic  if all the elements mono-of [ Y ] k  have the same colour,1 i.e. belong to the same of the  c  partition chromatic

classes of [ X] k. Similarly, if  G = ( V, E) is a graph and and all the edges of  H ⊆ G  have the same colour in some colouring of  E, we call  H  a  monochromatic subgraph  of  G, speak of a red (green, etc.)  H  in  G, and so on.

In the above terminology, Ramsey’s theorem can be expressed as follows: for every  r  there exists an  n  such that, given any  n-set  X, every 2-colouring of [ X]2 yields a monochromatic  r-set  Y ⊆ X. Interestingly, this assertion remains true for  c-colourings of [ X] k  with arbitrary  c and  k—with almost exactly the same proof!

To avoid repetition, we shall use this opportunity to demonstrate a

common alternative proof technique: we first prove an infinite version

of the general Ramsey theorem (which is easier, because we need not

worry about numbers), and then deduce the finite version by a so-called compactness argument.

1 Note that  Y  is called monochromatic, but it is the elements of [ Y ] k, not of  Y , that are (equally) coloured.

192

9. Ramsey Theory

[ 12.1.1 ]

Theorem 9.1.2.  Let k, c be positive integers, and X an infinite set. If

[ X] k is coloured with c colours, then X has an infinite monochromatic subset.

Proof . We prove the theorem by induction on  k, with  c  fixed. For  k = 1

the assertion holds, so let  k >  1 and assume the assertion for smaller values of  k.

Let [ X] k  be coloured with  c  colours. We shall construct an infinite sequence  X 0 , X 1 , . . .  of infinite subsets of  X  and choose elements  xi ∈ Xi with the following properties (for all  i):

(i)  Xi+1  ⊆ Xi  r  { xi };

(ii) all  k-sets  { xi } ∪ Z  with  Z ∈ [ Xi+1] k− 1 have the same colour, which we  associate  with  xi.

We start with  X 0 :=  X  and pick  x 0  ∈ X 0 arbitrarily. By assumption, X 0 is infinite. Having chosen an infinite set  Xi  and  xi ∈ Xi  for some  i, we  c-colour [ Xi  r  { xi }] k− 1 by giving each set  Z  the colour of  { xi } ∪ Z

from our  c-colouring of [ X] k. By the induction hypothesis,  Xi  r  { xi }

has an infinite monochromatic subset, which we choose as  Xi+1. Clearly, this choice satisfies (i) and (ii). Finally, we pick  xi+1  ∈ Xi+1 arbitrarily.

Since  c  is finite, one of the  c  colours is associated with infinitely many  xi. These  xi  form an infinite monochromatic subset of  X.

¤

To deduce the finite version of Theorem 9.1.2, we make use of a

standard graph-theoretical tool in combinatorics:

Lemma 9.1.3. (König’s Infinity Lemma)

Let V 0 , V 1 , . . . be an infinite sequence of disjoint non-empty finite sets, and let G be a graph on their union. Assume that every vertex v in a set Vn with n > 1  has a neighbour f ( v)  in Vn− 1 . Then G contains an infinite path v 0 v 1  . . . with vn ∈ Vn for all n.

f ( f ( v))

f ( v)

v

V 0

V 1

V 2

V 3

Fig. 9.1.2.  König’s infinity lemma

Proof . Let  P  be the set of all paths of the form  v f ( v)  f ( f ( v))  . . .  ending in  V 0. Since  V 0 is finite but  P  is infinite, infinitely many of the paths in P  end at the same vertex  v 0  ∈ V 0. Of these paths, infinitely many also agree on their penultimate vertex  v 1  ∈ V 1, because  V 1 is finite. Of those

9.1 Ramsey’s original theorems

193

paths, infinitely many agree even on their vertex  v 2 in  V 2—and so on.

Although the set of paths considered decreases from step to step, it is still infinite after any finite number of steps, so  vn  gets defined for every n ∈  N. By definition, each vertex  vn  is adjacent to  vn− 1 on one of those paths, so  v 0 v 1  . . .  is indeed an infinite path.

¤

Theorem 9.1.4.  For all k, c, r > 1  there exists an n >  k such that every

[ 9.3.3 ]

n-set X has a monochromatic r-subset with respect to any c-colouring of [ X] k.

Proof . As is customary in set theory, we denote by  n ∈  N (also) the set  {  0 , . . . , n −  1  }. Suppose the assertion fails for some  k, c, r. Then for k, c, r

every  n >  k  there exist an  n-set, without loss of generality the set  n, and a  c-colouring [ n] k → c  such that  n  contains no monochromatic  r-set. Let us call such colourings  bad ; we are thus assuming that for every  n >  k bad

colouring

there exists a bad colouring of [ n] k. Our aim is to combine these into a bad colouring of [N] k, which will contradict Theorem 9.1.2.

For every  n >  k  let  Vn 6=  ∅  be the set of bad colourings of [ n] k. For n > k, the restriction  f ( g) of any  g ∈ Vn  to [ n −  1] k  is still bad, and hence lies in  Vn− 1. By the infinity lemma, there is an infinite sequence gk, gk+1 , . . .  of bad colourings  gn ∈ Vn  such that  f ( gn) =  gn− 1 for all n > k. For every  m >  k, all colourings  gn  with  n >  m  agree on [ m] k, so for each  Y ∈ [N] k  the value of  gn( Y ) coincides for all  n >  max  Y . Let us define  g( Y ) as this common value  gn( Y ). Then  g  is a bad colouring of [N] k: every  r-set  S ⊆  N is contained in some sufficiently large  n, so  S  cannot be monochromatic since  g  coincides on [ n] k  with the bad colouring  gn.

¤

The least integer  n  associated with  k, c, r  as in Theorem 9.1.4 is the Ramsey

Ramsey number  for these parameters; we denote it by  R( k, c, r).

number

R( k, c, r)

9.2 Ramsey numbers

Ramsey’s theorem may be rephrased as follows: if  H =  Kr  and  G

is a graph with sufficiently many vertices, then either  G  itself or its complement  G  contains a copy of  H  as a subgraph. Clearly, the same is true for any graph  H, simply because  H ⊆ Kh  for  h :=  |H|.

However, if we ask for the  least n  such that every graph  G  with Ramsey

n  vertices has the above property—this is the  Ramsey number R( H) number

R( H)

of  H—then the above question makes sense: if  H  has only few edges, it should embed more easily in  G  or  G, and we would expect  R( H) to be smaller than the Ramsey number  R( h) =  R( Kh).

A little more generally, let  R( H 1 , H 2) denote the least  n ∈  N such R( H 1 , H 2)

that  H 1  ⊆ G  or  H 2  ⊆ G  for every graph  G  of order  n. For most graphs

194

9. Ramsey Theory

H 1 , H 2, only very rough estimates are known for  R( H 1 , H 2). Interestingly, lower bounds given by random graphs (as in Theorem 11.1.3) are often sharper than even the best bounds provided by explicit constructions.

The following proposition describes one of the few cases where exact

Ramsey numbers are known for a relatively large class of graphs:

Proposition 9.2.1.  Let s, t be positive integers, and let T be a tree of order t. Then R( T, Ks) = ( s −  1)( t −  1) + 1 .

(5.2.3)

Proof . The disjoint union of  s −  1 graphs  Kt− 1 contains no copy of  T ,

(1.5.4)

while the complement of this graph, the complete ( s −  1)-partite graph Ks− 1, does not contain  Ks. This proves  R( T, Ks) > ( s −  1)( t −  1) + 1.

t− 1Conversely, let  G  be any graph of order  n = ( s− 1)( t− 1)+1 whose complement contains no  Ks. Then  s >  1, and in any vertex colouring of  G (in the sense of Chapter 5) at most  s −  1 vertices can have the same colour. Hence,  χ( G) >  dn/( s −  1) e =  t. By Corollary 5.2.3,  G  has a subgraph  H  with  δ( H) >  t −  1, which by Corollary 1.5.4 contains a copy of  T .

¤

As the main result of this section, we shall now prove one of those

rare general theorems providing a relatively good upper bound for the

Ramsey numbers of a large class of graphs, a class defined in terms

of a standard graph invariant. The theorem deals with the Ramsey

numbers of sparse graphs: it says that the Ramsey number of graphs  H

with bounded maximum degree grows only linearly in  |H|—an enormous improvement on the exponential bound from the proof of Theorem 9.1.1.

Theorem 9.2.2. (Chvátal, Rödl, Szemerédi & Trotter 1983)

For every positive integer ∆  there is a constant c such that R( H) 6  c |H|

for all graphs H with ∆( H) 6 ∆ .

(7.1.1)

(7.2.1)

Proof . The basic idea of the proof is as follows. We wish to show that

(7.3.2)

(9.1.1)

H ⊆ G  or  H ⊆ G  if  |G|  is large enough (though not too large). Consider an  ²-regular partition of  G, as provided by the regularity lemma. If enough of the  ²-regular pairs in this partition have positive density, we may hope to find a copy of  H  in  G. If most pairs have zero or low density, we try to find  H  in  G. Let  R,  R0  and  R00  be the ‘regularity graphs’2 of G  whose edges correspond to the pairs of density > 0; > 1 / 2;  <  1 / 2; respectively. Then  R  is the edge-disjoint union of  R0  and  R00.

Now to obtain  H ⊆ G  or  H ⊆ G, it suffices by Lemma 7.3.2 to ensure that  H  is contained in a suitable ‘inflated regularity graph’  R0s 2 Later, we shall define  R00  a little differently, so that it complies with our formal definition of a regularity graph.

9.2 Ramsey numbers

195

or  R00s. Since  χ( H) 6 ∆( H) + 1 6 ∆ + 1, this will be the case if  s >  α( H) and we can find a  K∆+1 in  R0  or in  R00. But that is easy to ensure: we just need that  Kr ⊆ R, where  r  is the Ramsey number of ∆ + 1, which will follow from Tur´

an’s theorem because  R  is dense.

For the formal proof let now ∆ > 1 be given. On input  d := 1 / 2

∆ , d

and ∆, Lemma 7.3.2 returns an  ² 0; since the lemma’s assertion about  ² 0

² 0

becomes weaker if  ² 0 is made smaller, we may assume that  ² 0  <  1. Let m :=  R(∆ + 1) be the Ramsey number of ∆ + 1. Let  ²  6  ² 0 be positive m, ²

but small enough that, for  k =  m (and hence for all  k >  m), 1

2 ² <

−  1  .

(1)

m −  1

k

Finally, let  M  be the integer returned by the regularity lemma (7.2.1)

M

on input  ²  and  m.

All the quantities defined so far depend only on ∆. We shall prove

the theorem with

M

c :=

.

c

² 0 (1  − ²)

So let  H  with ∆( H) 6 ∆ be given, and let  s :=  |H|. Let  G  be an s

arbitrary graph of order  n >  c |H|; we show that  H ⊆ G  or  H ⊆ G.

G, n

By Lemma 7.2.1,  G  has an  ²-regular partition  { V 0 , V 1 , . . . , Vk }  with k

exceptional set  V 0 and  |V 1 | =  . . . =  |Vk| =:  `, where  m  6  k  6  M . Then

`

n − |V

1  − ²

1  − ²

s

` =

0 | >  n − ²n =  n

>  cs

=

.

(2)

k

M

M

M

² 0

Let  R  be the regularity graph with parameters  ², `,  0 corresponding to R

this partition. By definition,  R  has  k  vertices and

µ ¶

k

k

Rk >

− ²k 2

2 ³

´

= 1  k 2 1  −  1  −  2 ²

2

k

³

´

1

>  1  k 2 1  −  1  −

1

+

2

(1)

k

m −  1

k

m −  2

= 1  k 2

2

m −  1

>  tm− 1( k)

edges. By Theorem 7.1.1, therefore,  R  has a subgraph  K =  Km.

K

We now colour the edges of  R  with two colours: red if the edge corresponds to a pair ( Vi, Vj) of density at least 1 / 2, and green otherwise.

Let  R0  be the spanning subgraph of  R  formed by the red edges, and  R00

196

9. Ramsey Theory

the spanning subgraph of  R  formed by the green edges and those whose corresponding pair has density exactly 1 / 2. Then  R0  is a regularity graph of  G  with parameters  ²,  `  and 1 / 2. And  R00  is a regularity graph of  G, with the same parameters: as one easily checks, every pair ( Vi, Vj) that is  ²-regular for  G  is also  ²-regular for  G.

By definition of  m, our graph  K  contains a red or a green  Kr, for r

r :=  χ( H) 6 ∆ + 1. Correspondingly,  H ⊆ R0s  or  H ⊆ R00s. Since  ²  6  ² 0

and  ` >  s/² 0 by (2), both  R0  and  R00  satisfy the requirements of Lemma

7.3.2, so  H ⊆ G  or  H ⊆ G  as desired.

¤

So far in this section, we have been asking what is the least order of a graph  G  such that every 2-colouring of its edges yields a monochromatic copy of some given graph  H. Rather than focusing on the order of  G, we might alternatively try to minimize  G  itself, with respect to the subgraph Ramsey-relation. Given a graph  H, let us call a graph  G Ramsey-minimal  for  H

minimal

if  G  is minimal with the property that every 2-colouring of its edges yields a monochromatic copy of  H.

What do such Ramsey-minimal graphs look like? Are they unique?

The following result, which we include for its pretty proof, answers the second question for some  H:

Proposition 9.2.3.  If T is a tree but not a star, then infinitely many graphs are Ramsey-minimal for T .

(1.5.4)

(5.2.3)

Proof . Let  |T | =:  r. We show that for every  n ∈  N there is a graph of

(11.2.2)

order at least  n  that is Ramsey-minimal for  T .

Let us borrow the assertion of Theorem 11.2.2 from Chapter 11: by that theorem, there exists a graph  G  with chromatic number  χ( G)  > r 2

and girth  g( G)  > n. If we colour the edges of  G  red and green, then the red and the green subgraph cannot both have an  r-(vertex-)colouring in the sense of Chapter 5: otherwise we could colour the vertices of  G  with the pairs of colours from those colourings and obtain a contradiction

to  χ( G)  > r 2. So let  G0 ⊆ G  be monochromatic with  χ( G0)  > r. By

Corollary 5.2.3,  G0  has a subgraph of minimum degree at least  r, which contains a copy of  T  by Corollary 1.5.4.

Let  G∗ ⊆ G  be Ramsey-minimal for  T . Clearly,  G∗  is not a forest: the edges of any forest can be 2-coloured (partitioned) so that no monochromatic subforest contains a path of length 3, let alone a copy

of  T . (Here we use that  T  is not a star, and hence contains a  P  3.) So  G∗

contains a cycle, which has length  g( G)  > n  since  G∗ ⊆ G. In particular,

|G∗| > n  as desired.

¤

9.3 Induced Ramsey theorems

197

9.3 Induced Ramsey theorems

Ramsey’s theorem can be rephrased as follows. For every graph  H =  Kr there  exists  a graph  G  such that every 2-colouring of the edges of  G

yields a monochromatic  H ⊆ G; as it turns out, this is witnessed by any large enough complete graph as  G. Let us now change the problem slightly and ask for a graph  G  in which every 2-edge-colouring yields a monochromatic  induced H ⊆ G, where  H  is now an arbitrary given graph.

This slight modification changes the character of the problem dra-

matically. What is needed now is no longer a simple proof that  G  is

‘big enough’ (as for Theorem 9.1.1), but a careful construction: the construction of a graph that, however we bipartition its edges, contains an induced copy of  H  with all edges in one partition class. We shall call Ramsey

such a graph a  Ramsey graph  for  H.

graph

The fact that such a Ramsey graph exists for every choice of  H  is one of the fundamental results of graph Ramsey theory. It was proved

around 1973, independently by Deuber, by Erd˝

os, Hajnal & Pósa, and

by Rödl.

Theorem 9.3.1.  Every graph has a Ramsey graph. In other words, for every graph H there exists a graph G that, for every partition { E 1 , E 2  }

of E( G) , has an induced subgraph H with E( H)  ⊆ E 1  or E( H)  ⊆ E 2 .

We give two proofs. Each of these is highly individual, yet each offers a glimpse of true Ramsey theory: the graphs involved are used as hardly

more than bricks in the construction, but the edifice is impressive.

First proof. In our construction of the desired Ramsey graph we shall repeatedly replace vertices of a graph  G = ( V, E) already constructed by copies of another graph  H. For a vertex set  U ⊆ V  let  G [  U → H ]  G [  U → H ]

denote the graph obtained from  G  by replacing the vertices  u ∈ U  with copies  H( u) of  H  and joining each  H( u) completely to all  H( u0) with H( u)

uu0 ∈ E  and to all vertices  v ∈ V  r  U  with  uv ∈ E (Fig. 9.3.1). Formally, G

U

Fig. 9.3.1.  A graph  G [  U → H ] with  H =  K 3

198

9. Ramsey Theory

G [  U → H ] is the graph on

( U × V ( H))  ∪ (( V  r  U )  × { ∅ }) in which two vertices ( v, w) and ( v0, w0) are adjacent if and only if either vv0 ∈ E, or else  v =  v0 ∈ U  and  ww0 ∈ E( H).3

We prove the following formal strengthening of Theorem 9.3.1:

For any two graphs H 1 , H 2  there exists a graph G =

G( H 1 , H 2)

G( H 1 , H 2)  such that every edge colouring of G with the colours 1 and 2 yields either an induced H 1  ⊆ G with all

( ∗)

its edges coloured 1 or an induced H 2  ⊆ G with all its

edges coloured 2.

This formal strengthening makes it possible to apply induction on

|H 1 | +  |H 2 |, as follows.

If either  H 1 or  H 2 has no edges (in particular, if  |H 1 | +  |H 2 |  6 1), then ( ∗) holds with  G =  Kn  for large enough  n. For the induction step, we now assume that both  H 1 and  H 2 have at least one edge, and that ( ∗) holds for all pairs ( H0

|

|

1 , H 0 2) with smaller  |H 0 1 +  |H 0 2 .

xi

For each  i = 1 ,  2, pick a vertex  xi ∈ Hi  that is incident with an H0, H00

edge. Let  H0 :=  H

be the subgraph of  H0  induced by

i

i

i

i − xi, and let  H 00

i

i

the neighbours of  xi.

We shall construct a sequence  G 0 , . . . , Gn  of disjoint graphs;  Gn  will be the desired Ramsey graph  G( H 1 , H 2). Along with the graphs  Gi, we shall define subsets  V i ⊆ V ( Gi) and a map

f :  V  1  ∪ . . . ∪ V n → V  0  ∪ . . . ∪ V n− 1

such that

f ( V i) =  V i− 1

(1)

f i

for all  i > 1. Writing  f i :=  f ◦ . . . ◦ f  for the  i-fold composition of  f whenever it is defined, and  f  0 for the identity map on  V  0 =  V ( G 0), we origin

thus have  f i( v)  ∈ V  0 for all  v ∈ V i. We call  f i( v) the  origin  of  v.

The subgraphs  Gi [  V i ] will reflect the structure of  G 0 as follows: Vertices in V i with different origins are adjacent in Gi if

(2)

and only if their origins are adjacent in G 0 .

Assertion (2) will not be used formally in the proof below. However,

it can help us to visualize the graphs  Gi: every  Gi (more precisely, every Gi [  V i ]—there will also be some vertices  x ∈ Gi − V i) is essentially an inflated copy of  G 0 in which every vertex w ∈ G 0 has been replaced by 3 The replacement of  V  r  U  by ( V  r  U)  × { ∅ }  is just a formal device to ensure that all vertices of  G [  U → H ] have the same form ( v, w), and that  G [  U → H ] is formally disjoint from  G.

9.3 Induced Ramsey theorems

199

the set of all vertices in  V i  with origin  w, and the map  f  links vertices with the same origin across the various  Gi.

By the induction hypothesis, there are Ramsey graphs

G 1 :=  G( H 1 , H0 2) and  G 2 :=  G( H0 1 , H 2)  .

G 1 , G 2

Let  G 0 be a copy of  G 1, and set  V  0 :=  V ( G 0). Let  W 0 0 , . . . , W 0n− 1 be the G 0 , V  0

subsets of  V  0 spanning an  H0 2 in  G 0. Thus,  n  is defined as the number W 0i

of induced copies of  H0 2 in  G 0, and we shall construct a graph  Gi  for n

every set  W 0

,  i = 1 , . . . , n. Since  H

i− 1

1 has an edge,  n > 1: otherwise

G 0 could not be a  G( H 1 , H0 2). For  i = 0 , . . . , n −  1, let  W 00  be the image i

W 00

i

of  V ( H00

→

2 ) under some isomorphism  H 0 2

G 0 [  W 0 ].

i

Assume now that  G 0 , . . . , Gi− 1 and  V  0 , . . . , V i− 1 have been defined for some  i > 1, and that  f  has been defined on  V  1  ∪ . . . ∪ V i− 1 and satisfies (1) for all  j  6  i. We expand  Gi− 1 to  Gi  in two steps. For the first step, consider the set  U i− 1 of all the vertices  v ∈ V i− 1 whose origin U i− 1

f i− 1( v) lies in  W 00 . (For  i = 1, this gives  U  0 =  W 00

i− 1

0 .) Expand  Gi− 1

to a graph ˜

Gi− 1 by replacing every vertex  u ∈ U i− 1 with a copy  G 2( u) G 2( u)

of  G 2, i.e. let

˜

Gi− 1 :=  Gi− 1 [  U i− 1  → G 2 ]

˜

Gi− 1

(see Figures 9.3.2 and 9.3.3). Set  f ( u0) :=  u  for all  u ∈ U i− 1 and G 1

x( F )

H0 1( u)

H00

1 ( u)

v0

G 2( u)

V  1

u0

G 0

u

v

W 0 0

W 00

0

Fig. 9.3.2.  The construction of  G 1

u0 ∈ G 2( u), and  f ( v0) :=  v  for all  v0 = ( v, ∅) with  v ∈ V i− 1 r  U i− 1.

(Recall that ( v, ∅) is simply the unexpanded copy of a vertex  v ∈ Gi− 1

in ˜

Gi− 1.) Let  V i  be the set of those vertices  v0  or  u0  of ˜

Gi− 1 for which

V i

f  has thus been defined, i.e. the vertices that either correspond directly to a vertex  v  in  V i− 1 or else belong to an expansion  G 2( u) of such a vertex  u. Then (1) holds for  i. Also, if we assume (2) inductively for

200

9. Ramsey Theory

i −  1, then (2) holds again for  i (in ˜

Gi− 1). The graph ˜

Gi− 1 is already

the ‘essential part’ of  Gi: the part that looks like an inflated copy of  G 0.

In the second step we now extend ˜

Gi− 1 to the desired graph  Gi  by

F

adding some further vertices  x /

∈ V i. Let  F  denote the set of all families

F  of the form

¡

¢

F =  H0 1( u)  | u ∈ Ui− 1  ,

H0 ( u)

where each  H0

1

1( u) is an induced subgraph of  G 2( u) isomorphic to  H 0 1 .

(Less formally:  F  is the collection of ways to select from each  G 2( u) x( F )

exactly one induced copy of  H0 1.) For each  F ∈ F, add a vertex  x( F ) to ˜

Gi− 1 and join it to all the vertices of  H00

1 ( u) for every  u ∈ U i− 1,

H00( u)

where  H00

1

→

1 ( u) is the image of  H 00

1 under some isomorphism  H 0 1

H0 1( u)

Gi

(Fig. 9.3.2). Denote the resulting graph by  Gi. This completes the inductive definition of the graphs  G 0 , . . . , Gn.

Let us now show that  G :=  Gn  satisfies ( ∗). To this end, we prove the following assertion ( ∗∗) about  Gi  for  i = 0 , . . . , n: For every edge colouring with the colours 1 and 2, Gi contains either an induced H 1  coloured 1, or an induced H 2

coloured 2, or an induced subgraph H coloured 2 such that

( ∗∗)

V ( H)  ⊆ V i and the restriction of fi to V ( H)  is an isomorphism between H and G 0 [  W 0 ]  for some k ∈ { i, . . . , n −  1  }.

k

Note that the third of the above cases cannot arise for  i =  n, so ( ∗∗) for n  is equivalent to ( ∗) with  G :=  Gn.

For  i = 0, ( ∗∗) follows from the choice of  G 0 as a copy of  G 1 =

G( H 1 , H0 2) and the definition of the sets  W 0 . Now let 1 6  i  6  n, and k

assume ( ∗∗) for smaller values of  i.

Let an edge colouring of  Gi  be given. For each  u ∈ U i− 1 there is a copy of  G 2 in  Gi:

Gi ⊇ G 2( u)  ' G( H0 1 , H 2)  .

If  G 2( u) contains an induced  H 2 coloured 2 for some  u ∈ U i− 1, we are done. If not, then every  G 2( u) has an induced subgraph  H0 1( u)  ' H0 1

coloured 1. Let  F  be the family of these graphs  H0 1( u), one for each x

u ∈ U i− 1, and let  x :=  x( F ). If, for some  u ∈ U i− 1, all the  x– H00

1 ( u)

edges in  Gi  are also coloured 1, we have an induced copy of  H 1 in  Gi and are again done. We may therefore assume that each  H00

1 ( u) has a

yu

vertex  yu  for which the edge  xyu  is coloured 2. Let

ˆ

ˆ

U i− 1

U i− 1 :=  { yu | u ∈ U i− 1  } ⊆ V i.

Then  f  defines an isomorphism from

h

©

ª i

ˆ

ˆ

Gi− 1 :=  Gi

ˆ

U i− 1  ∪

( v, ∅)  | v ∈ V ( Gi− 1) r  U i− 1

Gi− 1

9.3 Induced Ramsey theorems

201

to  Gi− 1: just map every  yu  to  u  and every ( v, ∅) to  v. Our edge colouring of  Gi  thus induces an edge colouring of  Gi− 1. If this colouring yields an induced  H 1  ⊆ Gi− 1 coloured 1 or an induced  H 2  ⊆ Gi− 1 coloured 2, we have these also in ˆ

Gi− 1  ⊆ Gi  and are again home.

By ( ∗∗) for  i −  1 we may therefore assume that  Gi− 1 has an induced subgraph  H0  coloured 2, with  V ( H0)  ⊆ V i− 1, and such that the restric-H0

tion of  f i− 1 to  V ( H0) is an isomorphism from  H0  to  G 0 [  W 0 ]  ' H0

k

2

for some  k ∈ { i −  1 , . . . , n −  1  }. Let ˆ

H0  be the corresponding induced

ˆ

H0

subgraph of ˆ

Gi− 1  ⊆ Gi (also coloured 2); then  V ( ˆ

H0)  ⊆ V i,

f i( V ( ˆ

H0)) =  f i− 1( V ( H0)) =  W 0k ,

and  f i: ˆ

H0 → G 0 [  W 0 ] is an isomorphism.

k

x

Gi

H

ˆ  0

G

H

G

2

2( u)

y

ˆ  0

G 2  G 2

y

yu0

H

u

V i

U i− 1

H0

H0

u0

u

V i− 1

G 0

H0 2

W 0i− 1

W 00

i− 1

H 2

V  0

x 2

Fig. 9.3.3.  A monochromatic copy of  H 2 in  Gi

If  k >  i, this completes the proof of ( ∗∗) with  H := ˆ

H0; we therefore

assume that  k < i, and hence  k =  i −  1 (Fig. 9.3.3). By definition of  U i− 1 and ˆ

Gi− 1, the inverse image of  W 00

under the isomorphism

i− 1

f i: ˆ

H0 → G 0 [  W 0

] is a subset of ˆ

U i− 1. Since  x  is joined to precisely

i− 1

those vertices of ˆ

H0  that lie in ˆ

U i− 1, and all these edges  xyu  have colour 2,

the graph ˆ

H0  and  x  together induce in  Gi  a copy of  H 2 coloured 2, and the proof of ( ∗∗) is complete.

¤

202

9. Ramsey Theory

Let us return once more to the reformulation of Ramsey’s theorem

considered at the beginning of this section: for every graph  H  there exists a graph  G  such that every 2-colouring of the edges of  G  yields a monochromatic  H ⊆ G. The graph  G  for which this follows at once from Ramsey’s theorem is a sufficiently large complete graph. If we ask, however, that  G  shall not contain any complete subgraphs larger than those in  H, i.e. that  ω( G) =  ω( H), the problem again becomes difficult—even if we do not require  H  to be induced in  G.

Our second proof of Theorem 9.3.1 solves both problems at once: given  H, we shall construct a Ramsey graph for  H  with the same clique number as  H.

For this proof, i.e. for the remainder of this section, let us view

bipartite

bipartite graphs  P  as triples ( V 1 , V 2 , E), where  V 1 and  V 2 are the two vertex classes and  E ⊆ V 1  × V 2 is the set of edges. The reason for this more explicit notation is that we want embeddings between bipartite

graphs to respect their bipartitions: given another bipartite graph  P 0 =

( V 0

∪

1  , V 0

2  , E0), an injective map  ϕ:  V 1  ∪ V 2  → V 0

1

V 0 2 will be called an

embedding

embedding  of  P  in  P 0  if  ϕ( Vi)  ⊆ V 0  for  i = 1 ,  2 and  ϕ( v P

i

1) ϕ( v 2) is an edge

→ P 0

of  P 0  if and only if  v 1 v 2 is an edge of  P . (Note that such embeddings are ‘induced’.) Instead of  ϕ:  V 1  ∪ V 2  → V 0 ∪

1

V 0 2 we may simply write

ϕ:  P → P 0.

We need two lemmas.

Lemma 9.3.2.  Every bipartite graph can be embedded in a bipartite E

graph of the form ( X, [ X] k, E)  with E =  { xY | x ∈ Y }.

Proof . Let  P  be any bipartite graph, with vertex classes  { a 1 , . . . , an }

and  { b 1 , . . . , bm }, say. Let  X  be a set with 2 n +  m  elements, say X =  { x 1 , . . . , xn, y 1 , . . . , yn, z 1 , . . . , zm } ; we shall define an embedding  ϕ:  P → ( X, [ X] n+1 , E).

Let us start by setting  ϕ( ai) :=  xi  for all  i = 1 , . . . , n. Which ( n + 1)-sets  Y ⊆ X  are suitable candidates for the choice of  ϕ( bi) for a given vertex  bi? Clearly those adjacent exactly to the images of the neighbours of  bi, i.e. those satisfying

Y ∩ { x 1 , . . . , xn } =  ϕ( NP ( bi))  .

(1)

Since  d( bi) 6  n, the requirement of (1) leaves at least one of the  n + 1

elements of  Y  unspecified. In addition to  ϕ( NP ( bi)), we may therefore include in each  Y =  ϕ( bi) the vertex  zi  as an ‘index’; this ensures that ϕ( bi)  6=  ϕ( bj) for  i 6=  j, even when  bi  and  bj  have the same neighbours in  P . To specify the sets  Y =  ϕ( bi) completely, we finally fill them up with ‘dummy’ elements  yj  until  |Y | =  n + 1.

¤

9.3 Induced Ramsey theorems

203

Our second lemma already covers the bipartite case of the theorem:

it says that every bipartite graph has a Ramsey graph—even a bipartite

one.

Lemma 9.3.3.  For every bipartite graph P there exists a bipartite graph P 0 such that for every 2-colouring of the edges of P 0 there is an embedding ϕ:  P → P 0 for which all the edges of ϕ( P )  have the same colour.

Proof . We may assume by Lemma 9.3.2 that  P  has the form ( X, [ X] k, E)

(9.1.4)

with  E =  { xY | x ∈ Y }. We show the assertion for the graph  P 0 :=

P, X, k, E

( X0, [ X0] k0 , E0), where  k0 := 2 k −  1,  X0  is any set of cardinality P 0, X0, k0

³

¡ ¢

´

|X0| =  R k0,  2  k0 , k |X| +  k −  1  , k

(this is the Ramsey number defined after Theorem 9.1.4), and E0 :=  { x0Y 0 | x0 ∈ Y 0 } .

E0

Let us then colour the edges of  P 0  with two colours  α  and  β. Of the α, β

|Y 0| = 2 k −  1 edges incident with a vertex  Y 0 ∈ [ X0] k0, at least  k  must have the same colour. For each  Y 0  we may therefore choose a fixed  k-set Z0 ⊆ Y 0  such that all the edges  x0Y 0  with  x0 ∈ Z0  have the same colour; Z0

we shall call this colour  associated  with  Y 0.

associated

¡ ¢

The sets  Z0  can lie within their supersets  Y 0  in  k0  ways, as follows.

k

Let  X0  be linearly ordered. Then for every  Y 0 ∈ [ X0] k0  there is a unique order-preserving bijection  σY 0:  Y 0 → {  1 , . . . , k0 }, which maps  Z0  to one σY 0

¡ ¢

of  k0  possible images.

k

¡ ¢

We now colour [ X0] k0  with the 2  k0  elements of the set k

[ {  1 , . . . , k0 }] k × { α, β }

as colours, giving each  Y 0 ∈ [ X0] k0  as its colour the pair ( σY 0( Z0) , γ), where  γ  is the colour  α  or  β  associated with  Y 0. Since  |X0|  was chosen

¡ ¢

as the Ramsey number with parameters  k0, 2  k0  and  k |X| +  k −  1, we k

know that  X0  has a monochromatic subset  W  of cardinality  k |X| +  k −  1.

W

All  Z0  with  Y 0 ⊆ W  thus lie within their  Y 0  in the same way, i.e. there exists an  S ∈ [ {  1 , . . . , k0 }] k  such that  σY 0( Z0) =  S  for all  Y 0 ∈ [ W ] k0, and all  Y 0 ∈ [ W ] k0  are associated with the same colour, say with  α.

α

We now construct the desired embedding  ϕ  of  P  in  P 0. We first ϕ|X

define  ϕ  on  X =:  { x 1 , . . . , xn }, choosing images  ϕ( xi) =:  wi ∈ W  so xi, wi, n

that  wi < wj  in our ordering of  X0  whenever  i < j. Moreover, we choose the  wi  so that exactly  k −  1 elements of  W  are smaller than  w 1, exactly k −  1 lie between  wi  and  wi+1 for  i = 1 , . . . , n −  1, and exactly  k −  1

are bigger than  wn. Since  |W | =  kn +  k −  1, this can indeed be done

(Fig. 9.3.4).

204

9. Ramsey Theory

...

n

k −  1

w 1

Y 0

w 2

Z0

...

n

k −  1

˜

Z0

n

˜

k −  1

Y 0

wn

n

k −  1

W

...

X0

Fig. 9.3.4.  The graph of Lemma 9.3.3

ϕ|[ X] k

We now define  ϕ  on [ X] k. Given  Y ∈ [ X] k, we wish to choose ϕ( Y ) =:  Y 0 ∈ [ X0] k0  so that the neighbours of  Y 0  among the vertices in  ϕ( X) are precisely the images of the neighbours of  Y  in  P , i.e. the vertices  ϕ( x) with  x ∈ Y , and so that all these edges at  Y 0  are coloured  α.

To find such a set  Y 0, we first fix its subset  Z0  as  { ϕ( x)  | x ∈ Y }

(these are  k  vertices of type  wi) and then extend  Z0  by  k0 − k  further vertices  u ∈ W  r  ϕ( X) to a set  Y 0 ∈ [ W ] k0, in such a way that  Z0  lies correctly within  Y 0, i.e. so that  σY 0( Z0) =  S. This can be done, because k −  1 =  k0 − k  other vertices of  W  lie between any two  wi. Then Y 0 ∩ ϕ( X) =  Z0 =  { ϕ( x)  | x ∈ Y } , so  Y 0  has the correct neighbours in  ϕ( X), and all the edges between  Y 0

and these neighbours are coloured  α (because those neighbours lie in  Z0

and  Y 0  is associated with  α). Finally,  ϕ  is injective on [ X] k: the images Y 0  of different vertices  Y  are distinct, because their intersections with ϕ( X) differ. Hence, our map  ϕ  is indeed an embedding of  P  in  P 0.

¤

9.3 Induced Ramsey theorems

205

Second proof of Theorem 9.3.1. Let  H  be given as in the theorem, and let  n :=  R( r) be the Ramsey number of  r :=  |H|. Then, for every r, n

2-colouring of its edges, the graph  K =  Kn  contains a monochromatic K

copy of  H—although not necessarily induced.

We start by constructing a graph  G 0, as follows. Imagine the vertices of  K  to be arranged in a column, and replace every vertex by a row

¡ ¢

¡ ¢

of  n  vertices. Then each of the  n  columns arising can be associated r

¡ ¢

r

with one of the  n  ways of embedding  V ( H) in  V ( K); let us furnish r

this column with the edges of such a copy of  H. The graph  G 0 thus aris-

¡ ¢

¡ ¢

ing consists of  n  disjoint copies of  H  and ( n − r)  n  isolated vertices r

r

(Fig. 9.3.5).

¡ ¢

n

|

{z

r

} }

H

H

H

r

H

. . .

n

{z

n − r

|

Fig. 9.3.5.  The graph  G 0

In order to define  G 0 formally, we assume that  V ( K) =  {  1 , . . . , n }

and choose copies  H 1 , . . . , H( n) of  H  in  K  with pairwise distinct vertex r

sets. (Thus, on each  r-set in  V ( K) we have one fixed copy  Hj  of  H.) We then define

©

¡ ¢ª

V ( G 0) := ( i, j)  | i = 1 , . . . , n;  j = 1 , . . . , nr ( n)

r

[ ©

ª

G 0

E( G 0) :=

( i, j)( i0, j)  | ii0 ∈ E( Hj)  .

j=1

The idea of the proof now is as follows. Our aim is to reduce the

general case of the theorem to the bipartite case dealt with in Lem-

ma 9.3.3. Applying the lemma iteratively to all the pairs of rows of  G 0, we construct a very large graph  G  such that for every edge colouring of  G  there is an induced copy of  G 0 in  G  that is monochromatic on all the bipartite subgraphs induced by its pairs of rows, i.e. in which edges between the same two rows always have the same colour. The projection

of this  G 0  ⊆ G  to  {  1 , . . . , n } (by contracting its rows) then defines an edge colouring of  K. By the choice of  |K|, one of the  Hj ⊆ K  will be monochromatic. But this  Hj  occurs with the same colouring in the  j th column of our  G 0, where it is an induced subgraph of  G 0, and hence of  G.

206

9. Ramsey Theory

Formally, we shall define a sequence  G 0 , . . . , Gm  of  n-partite graphs Gk, with  n-partition  { V k

}

1  , . . . , V k

n

say, and then let  G :=  Gm. The

graph  G 0 has been defined above; let  V  0

1  , . . . , V  0

n  be its rows:

©

¡ ¢ª

V  0

V  0

n

i

i

:= ( i, j)  | j = 1 , . . . ,

.

r

ek, m

Now let  e 1 , . . . , em  be an enumeration of the edges of  K. For  k =

i 1 , i 2

0 , . . . , m −  1, construct  Gk+1 from  Gk  as follows. If  ek+1 =  i 1 i 2, say, P

let  P = ( V k, V k, E) be the bipartite subgraph of  Gk  induced by its i 1

i 2

P 0

i 1th and  i 2th row. By Lemma 9.3.3,  P  has a bipartite Ramsey graph W 1 , W 2

P 0 = ( W 1 , W 2 , E0). We wish to define  Gk+1  ⊇ P 0  in such a way that every (monochromatic) embedding  P → P 0  can be extended to an embedding ϕp, q

Gk → Gk+1. Let  { ϕ 1 , . . . , ϕq }  be the set of all embeddings of  P  in  P 0, and let

V ( Gk+1) :=  V k+1  ∪ . . . ∪ V k+1

1

n

,

where



  W 1

for  i =  i 1

V k+1 :=

W

i

 2

for  i =  i 2

S q ( V k ×{p}) for  i /∈ {i

p=1

i

1 , i 2  }.

(Thus for  i 6=  i 1 , i 2, we take as  V k+1 just  q  disjoint copies of  V k.) We i

i

now define the edge set of  Gk+1 so that the obvious extensions of  ϕp  to all of  V ( Gk) become embeddings of  Gk  in  Gk+1: for  p = 1 , . . . , q, let ψp:  V ( Gk)  → V ( Gk+1) be defined by ½  ϕ

ψ

p( v)

for  v ∈ P

p( v) :=

( v, p)

for  v /

∈ P

and let

q

[

E( Gk+1) :=

{ ψp( v) ψp( v0)  | vv0 ∈ E( Gk)  } .

p=1

Now for every 2-colouring of its edges,  Gk+1 contains an induced copy ψp( Gk) of  Gk  whose edges in  P , i.e. those between its  i 1th and  i 2th row, have the same colour: just choose  p  so that  ϕp( P ) is the monochromatic induced copy of  P  in  P 0  that exists by Lemma 9.3.3.

We claim that  G :=  Gm  satisfies the assertion of the theorem. So let a 2-colouring of the edges of  G  be given. By the construction of Gm  from  Gm− 1, we can find in  Gm  an induced copy of  Gm− 1 such that for  em =  ii0  all edges between the  i th and the  i0 th row have the same colour. In the same way, we find inside this copy of  Gm− 1 an induced copy of  Gm− 2 whose edges between the  i th and the  i0 th row have the same colour also for  ii0 =  em− 1. Continuing in this way, we finally arrive at an induced copy of  G 0 in  G  such that, for each pair ( i, i0), all the edges between  V  0 and  V  0

i

i0  have the same colour. As shown earlier, this

G 0 contains a monochromatic induced copy  Hj  of  H.

¤

9.4. Ramsey properties and connectivity

207

9.4 Ramsey properties and connectivity

According to Ramsey’s theorem, every large enough graph  G  has a very dense or a very sparse induced subgraph of given order, a  Kr  or  Kr. If we assume that  G  is connected, we can say a little more:

Proposition 9.4.1.  For every r ∈  N  there is an n ∈  N  such that every connected graph of order at least n contains Kr, K 1 ,r or P r as an induced subgraph.

Proof . Let  d + 1 be the Ramsey number of  r, let  n >  1 +  rd r, and let  G

be a graph of order at least  n. If  G  has a vertex  v  of degree at least  d + 1

then, by Theorem 9.1.1 and the choice of  d, either  N ( v) induces a  Kr  in G  or  { v } ∪ N ( v) induces a  K 1 ,r. On the other hand, if ∆( G) 6  d, then by Proposition 1.3.3  G  has radius  > r, and hence contains two vertices at a distance >  r. Any shortest path in  G  between these two vertices contains a  P r.

¤

The collection of ‘typical’ induced subgraphs in Proposition 9.4.1

is smallest possible in the following sense. If  G  is  any  set of connected graphs with the same property, i.e. such that, given  r ∈  N, every large enough connected graph  G  contains an induced copy of a graph of order >  r  from  G, then  G  contains arbitrarily large complete graphs, stars and paths. (Note that if we take a complete graph, a star or a path as  G, and then all its subgraphs are again of that type.) But Proposition 9.4.1

tells us that we need no more than these.

In principle, we could look for a set like  G  for any assumed connectivity  k. We could try to find a ‘minimal’ set (in the above sense) of typical k-connected graphs, one such that every large  k-connected graph has a large subgraph in this set. Unfortunately,  G  seems to grow very quickly with  k: already for  k = 2 it becomes thoroughly messy if (as for  k = 1) we insist that those subgraphs be induced. By relaxing our specification of containment from ‘induced subgraph’ to ‘topological minor’ and on to

‘minor’, however, we can give some neat characterizations up to  k = 4.

Proposition 9.4.2.  For every r ∈  N  there is an n ∈  N  such that every 2-connected graph of order at least n contains Cr or K 2 ,r as a topological minor.

Proof . Let  d  be the  n  associated with  r  in Proposition 9.4.1, and let  G

(1.3.3)

(3.3.5)

be a 2-connected graph with more than 1 +  rd r  vertices. By Proposition

1.3.3, either  G  has a vertex of degree  > d  or diam( G) > rad( G)  > r.

In the latter case let  a, b ∈ G  be two vertices at distance  > r. By

Menger’s theorem (3.3.5),  G  contains two independent  a– b  paths. These form a cycle of length  > r.

208

9. Ramsey Theory

Assume now that  G  has a vertex  v  of degree  > d. Since  G  is 2-connected,  G − v  is connected and thus has a spanning tree; let  T  be a minimal tree in  G − v  that contains all the neighbours of  v. Then every leaf of  T  is a neighbour of  v. By the choice of  d, either  T  has a vertex of degree >  r  or  T  contains a path of length >  r, without loss of generality linking two leaves. Together with  v, such a path forms a cycle of length >  r. A vertex  u  of degree >  r  in  T  can be joined to  v  by  r independent paths through  T , to form a  T K 2 ,r.

¤

Theorem 9.4.3. (Oporowski, Oxley & Thomas 1993)

For every r ∈  N  there is an n ∈  N  such that every 3-connected graph of order at least n contains a wheel of order r or a K 3 ,r as a minor.

Let us call a graph of the form  Cn ∗ K 2 ( n > 4) a  double wheel, the 1-skeleton of a triangulation of the cylinder as in Fig. 9.4.1 a  crown, and the 1-skeleton of a triangulation of the Möbius strip a  M¨

obius crown.

Fig. 9.4.1.  A crown and a Möbius crown

Theorem 9.4.4. (Oporowski, Oxley & Thomas 1993)

For every r ∈  N  there is an n ∈  N  such that every 4-connected graph with at least n vertices has a minor of order >  r that is a double wheel, a crown, a Möbius crown, or a K 4 ,s.

Note that the minors occurring in Theorems 9.4.3 and 9.4.4 are them-

selves 3- and 4-connected, respectively, and are not minors of one anoth-er. Thus in each case, the collection of minors is minimal in the sense discussed earlier.

Exercises

1.  −  Determine the Ramsey number  R(3).

2.

Deduce the case  k = 2 (but  c  arbitrary) of Theorem 9.1.4 directly from

Theorem 9.1.1.

3.+ Construct a graph on R that has neither a complete nor an edgeless

induced subgraph on  | R | = 2 ℵ 0 vertices. (So Ramsey’s theorem does not extend to uncountable sets.)

4.+ Use Ramsey’s theorem to show that for any  k, ` ∈  N there is an  n ∈  N

such that every sequence of  n  distinct integers contains an increasing subsequence of length  k + 1 or a decreasing subsequence of length  ` + 1.

Find an example showing that  n > k`. Then prove the theorem of Erd˝

os and Szekeres that  n =  k` + 1 will do.

Exercises

209

5.

Sketch a proof of the following theorem of Erd˝

os and Szekeres: for every

k ∈  N there is an  n ∈  N such that among any  n  points in the plane, no three of them collinear, there are  k  points spanning a convex  k-gon, i.e. such that none of them lies in the convex hull of the others.

6.

Prove the following result of Schur: for every  k ∈  N there is an  n ∈  N

such that, for every partition of  {  1 , . . . , n }  into  k  sets, at least one of the subsets contains numbers  x, y, z  such that  x +  y =  z.

7.

Let ( X,  6) be a totally ordered set, and let  G = ( V, E) be the graph on  V := [ X]2 with  E :=  {( x, y)( x0, y0)  | x < y =  x0 < y0}.

(i) Show that  G  contains no triangle.

(ii) Show that  χ( G) will get arbitrarily large if  |X|  is chosen large enough.

8.

A family of sets is called a ∆ -system  if every two of the sets have the same intersection. Show that every infinite family of sets of the same

finite cardinality contains an infinite ∆-system.

9.

Prove the following weakening of Scott’s Theorem 8.1.5: for every  r ∈  N

and every tree  T  there exists a  k ∈  N such that every graph  G  with χ( G) >  k  and  ω( G)  < r  contains a subdivision of  T  in which no two branch vertices are adjacent in  G (unless they are adjacent in  T ).

10.

Use the infinity lemma to show that, given  k ∈  N, a countably infinite graph is  k-colourable (in the sense of Chapter 5) if all its finite subgraphs are  k-colourable.

11.

Let  m, n ∈  N, and assume that  m −  1 divides  n −  1. Show that every tree  T  of order  m  satisfies  R( T, K 1 ,n) =  m +  n −  1.

12.

Prove that 2 c < R(2 , c,  3) 6 3 c! for every  c ∈  N.

(Hint. Induction on  c.)

13.  −  Derive the statement ( ∗) in the first proof of Theorem 9.3.1 from the theorem itself, i.e. show that ( ∗) is only formally stronger than the theorem.

14.

Show that the Ramsey graph  G  for  H  constructed in the second proof

of Theorem 9.3.1 does indeed satisfy  ω( G) =  ω( H).

15.

Show that, given any two graphs  H 1 and  H 2, there exists a graph G =  G( H 1 , H 2) such that, for every  vertex-colouring of  G  with colours 1 and 2, there is either an induced copy of  H 1 coloured 1 or an induced copy of  H 2 coloured 2 in  G.

(Hint. Apply induction as in the first proof of Theorem 9.3.1.)

16.

Show that every infinite connected graph contains an infinite path or

an infinite star.

17.  −  The  Kr  from Ramsey’s theorem, last sighted in Proposition 9.4.1, conspicuously fails to make an appearance from Proposition 9.4.2 onwards.

Can it be excused?

210

9. Ramsey Theory

Notes

Due to increased interaction with research on random and pseudo-random4

structures (the latter being provided, for example, by the regularity lemma),

the Ramsey theory of graphs has recently seen a period of major activity and advance. Theorem 9.2.2 is an early example of this development.

For the more classical approach, the introductory text by R.L. Graham,

B.L. Rothschild & J.H. Spencer,  Ramsey Theory (2nd edn.), Wiley 1990, makes stimulating reading. This book includes a chapter on graph Ramsey theory, but is not confined to it. A more recent general survey is given by J. Nešetřil in the  Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995. The Ramsey theory of infinite sets forms a substantial part of combinatorial set theory, and is treated in depth in P. Erd˝

os, A. Hajnal, A. Máté & R. Rado,  Combinatorial Set Theory, North-Holland 1984. An attractive collection of highlights from various branches of Ramsey theory, including applications in algebra, geometry and point-set topology, is offered in B. Bollobás,  Graph Theory, Springer GTM 63, 1979.

König’s infinity lemma, or  K¨

onig’s lemma for short, is contained in

the first-ever book on the subject of graph theory: D. König,  Theorie der endlichen und unendlichen Graphen, Akademische Verlagsgesellschaft, Leipzig 1936. The  compactness  technique for deducing finite results from infinite (or vice versa), hinted at in Section 9.1, is less mysterious than it sounds. As long as ‘infinite’ means ‘countably infinite’, it is precisely the art of applying the infinity lemma (as in the proof of Theorem 9.1.4), no more no less. For larger infinite sets, the same argument becomes equivalent to the well-known theorem of Tychonov that arbitrary products of compact spaces are compact—

which has earned the compactness argument its name. Details can be found in Ch. 6, Thm. 10 of Bollobás, and in Graham, Rothschild & Spencer, Ch. 1, Thm. 4. Another frequently used version of the general compactness argument is  Rado’s selection lemma; see A. Hajnal’s chapter on infinite combinatorics in the Handbook cited above.

Theorem 9.2.2 is due to V. Chvátal, V. Rödl, E. Szemerédi & W.T. Trotter, The Ramsey number of a graph with bounded maximum degree,  J. Combin.

Theory B 34 (1983), 239–243. Our proof follows the sketch in J. Komlós & M. Simonovits, Szemerédi’s Regularity Lemma and its applications in graph theory, in (D. Miklós, V.T. Sós & T. Sz˝

onyi, eds.)  Paul Erd˝

os is 80, Vol. 2,

Proc. Colloq. Math. Soc. János Bolyai (1996). The theorem marks a break-through towards a conjecture of Burr and Erd˝

os (1975), which asserts that the

Ramsey numbers of graphs with bounded  average  degree in every subgraph are linear: for every  d ∈  N, the conjecture says, there exists a constant  c  such that R( H) 6  c |H|  for all graphs  H  with  d( H0) 6  d  for all  H0 ⊆ H. This conjecture has been verified also for the class of planar graphs (Chen & Schelp 1993) and, more generally, for the class of graphs not containing  Kr (for any fixed  r) as a topological minor (Rödl & Thomas 1996). See Nešetřil’s Handbook chapter for references.

4 Concrete graphs whose structure resembles the structure expected of a random graph are called  pseudo-random. For example, the bipartite graphs spanned by an ²-regular pair of vertex sets in a graph are pseudo-random.

Notes

211

Our first proof of Theorem 9.3.1 is based on W. Deuber, A generalization of Ramsey’s theorem, in (A. Hajnal, R. Rado & V.T. Sós, eds.)  Infinite and finite sets, North-Holland 1975. The same volume contains the alternative proof of this theorem by Erd˝

os, Hajnal and Pósa. Rödl proved the same result

in his MSc thesis at the Charles University, Prague, in 1973. Our second proof of Theorem 9.3.1, which preserves the clique number of  H  for  G, is due to J. Nešetřil & V. Rödl, A short proof of the existence of restricted Ramsey graphs by means of a partite construction,  Combinatorica 1 (1981), 199–202.

The two theorems in Section 9.4 are due to B. Oporowski, J. Oxley & R. Thomas, Typical subgraphs of 3- and 4-connected graphs,  J. Combin. Theory B 57 (1993), 239–257.

10

Hamilton Cycles

In Chapter 1.8 we briefly discussed the problem of when a graph contains an Euler tour, a closed walk traversing every edge exactly once. The

simple Theorem 1.8.1 solved that problem quite satisfactorily. Let us now ask the analogous question for vertices: when does a graph  G  contain a closed walk that contains every vertex of  G  exactly once? If  |G| > 3, Hamilton

then any such walk is a cycle: a  Hamilton cycle  of  G. If  G  has a Hamilton cycle

cycle, it is called  hamiltonian. Similarly, a path in  G  containing every vertex of  G  is a  Hamilton path.

Hamilton

path

To determine whether or not a given graph has a Hamilton cycle is

much harder than deciding whether it is Eulerian, and no good charac-

terization1 is known of the graphs that do. We shall begin this chapter by presenting the standard sufficient conditions for the existence of a Hamilton cycle (Sections 10.1 and 10.2). The rest of the chapter is then devoted to the beautiful theorem of Fleischner that the ‘square’ of every 2-connected graph has a Hamilton cycle. This is one of the main results in the field of Hamilton cycles. The simple proof we present (due to Ř´ıha) is still a little longer than other proofs in this book, but not difficult.

10.1 Simple sufficient conditions

What kind of condition might be sufficient for the existence of a Hamilton cycle in a graph  G? Purely global assumptions, like high edge density, will not be enough: we cannot do without the local property that every

vertex has at least two neighbours. But neither is any large (but con-

stant) minimum degree sufficient: it is easy to find graphs without a Hamilton cycle whose minimum degree exceeds any given constant bound.

1 The notion of a ‘good characterization’ can be made precise; see the introduction to Chapter 12.5 and the notes for Chapter 12.

214

10. Hamilton Cycles

The following classic result derives its significance from this back-

ground:

Theorem 10.1.1. (Dirac 1952)

Every graph with n > 3  vertices and minimum degree at least n/ 2  has a Hamilton cycle.

Proof . Let  G = ( V, E) be a graph with  |G| =  n > 3 and  δ( G) >  n/ 2.

Then  G  is connected: otherwise, the degree of any vertex in the smallest component  C  of  G  would be less than  |C|  6  n/ 2.

Let  P =  x 0  . . . xk  be a longest path in  G. By the maximality of  P , all the neighbours of  x 0 and all the neighbours of  xk  lie on  P . Hence at least  n/ 2 of the vertices  x 0 , . . . , xk− 1 are adjacent to  xk, and at least n/ 2 of these same  k < n  vertices  xi  are such that  x 0 xi+1  ∈ E. By the pigeon hole principle, there is a vertex  xi  that has both properties, so we have  x 0 xi+1  ∈ E  and  xixk ∈ E  for some  i < k (Fig. 10.1.1).

xi+1

. . .

. . .

x 0

xi

xk

P

Fig. 10.1.1.  Finding a Hamilton cycle in the proof Theorem 10.1.1

We claim that the cycle  C :=  x 0 xi+1 P xkxiP x 0 is a Hamilton cycle of  G. Indeed, since  G  is connected,  C  would otherwise have a neighbour in  G − C, which could be combined with a spanning path of  C  into a path longer than  P .

¤

Theorem 10.1.1 is best possible in that we cannot replace the bound

of  n/ 2 with  bn/ 2 c: if  n  is odd and  G  is the union of two copies of  Kdn/ 2 e meeting in one vertex, then  δ( G) =  bn/ 2 c  but  κ( G) = 1, so  G  cannot have a Hamilton cycle. In other words, the high level of the bound of

δ >  n/ 2 is needed to ensure, if nothing else, that  G  is 2-connected: a condition just as trivially necessary for hamiltonicity as a minimum

degree of at least 2. It would seem, therefore, that prescribing some

high (constant) value for  κ  rather than for  δ  stands a better chance of implying hamiltonicity. However, this is not so: although  k-connected graphs contain long cycles in terms of  k (Ex. 14, Ch. 3), the graphs  Kn,k show that their circumference need not grow with  n.

There is another invariant with a similar property: a low inde-

pendence number  α( G) ensures that  G  has long cycles (Ex. 13, Ch. 5),

though not necessarily a Hamilton cycle. Put together, however, the

two assumptions of high connectivity and low independence number

surprisingly complement each other to produce a sufficient condition for hamiltonicity:

10.1 Simple sufficient conditions

215

Proposition 10.1.2.  Every graph G with |G| > 3  and κ( G) >  α( G) has a Hamilton cycle.

Proof . Put  κ( G) =:  k, and let  C  be a longest cycle in  G. Enumerate the

(3.3.3)

vertices of  C  cyclically, say as  V ( C) =  { vi | i ∈  Z n }  with  vivi+1  ∈ E( C) k

for all  i ∈  Z n. If  C  is not a Hamilton cycle, pick a vertex  v ∈ G − C  and a  v– C  fan  F =  { Pi | i ∈ I }  in  G, where  I ⊆  Z n  and each  Pi  ends in  vi.

Let  F  be chosen with maximum cardinality; then  vvj /

∈ E( G) for any

j /

∈ I, and

|F| > min  { k, |C| }

(1)

by Menger’s theorem (3.3.3).

For every  i ∈ I, we have  i + 1  /

∈ I: otherwise, ( C ∪ Pi ∪ Pi+1)  − vivi+1

would be a cycle longer than  C (Fig. 10.1.2, left). Thus  |F| < |C|, and hence  |I| =  |F| >  k  by (1). Furthermore,  vi+1 vj+1  /

∈ E( G) for all  i, j ∈ I,

as otherwise ( C ∪ Pi ∪ Pj) +  vi+1 vj+1  − vivi+1  − vjvj+1 would be a cycle longer than  C (Fig. 10.1.2, right). Hence  { vi+1  | i ∈ I } ∪ { v }  is a set of k + 1 or more independent vertices in  G, contradicting  α( G) 6  k.

¤

v

v

Pj

Pi+1

F

vj+1

v

vj

P

i+1

P

i

i

vi+1

vi

vi

C

C

Fig. 10.1.2.  Two cycles longer than  C

It may come as a surprise to learn that hamiltonicity for planar

graphs is related to the four colour problem. As we noted in Chapter 6.6, the four colour theorem is equivalent to the non-existence of a planar snark, i.e. to the assertion that every bridgeless planar cubic graph has a 4-flow. It is easily checked that ‘bridgeless’ can be replaced with ‘3-connected’ in this assertion, and that every hamiltonian graph has a

4-flow (Ex. 12, Ch. 6). For a proof of the four colour theorem, therefore, it would suffice to show that every 3-connected planar cubic graph has

a Hamilton cycle!

Unfortunately, this is not the case: the first counterexample was

found by Tutte in 1946. Ten years later, Tutte proved the following

deep theorem as a best possible weakening:

Theorem 10.1.3. (Tutte 1956)

Every 4-connected planar graph has a Hamilton cycle.

216

10. Hamilton Cycles

10.2 Hamilton cycles and degree sequences

Historically, Dirac’s theorem formed the point of departure for the dis-covery of a series of weaker and weaker degree conditions, all sufficient for hamiltonicity. The development culminated in a single theorem that

encompasses all the earlier results: the theorem we shall prove in this section.

If  G  is a graph with  n  vertices and degrees  d 1 6  . . .  6  dn, then the degree

n-tuple ( d 1 , . . . , dn) is called the  degree sequence  of  G. Note that this sequence

sequence is unique, even though  G  has several vertex enumerations giving rise to its degree sequence. Let us call an arbitrary integer sequence

hamiltonian ( a 1 , . . . , an)  hamiltonian  if every graph with  n  vertices and a degree sequence

sequence pointwise greater than ( a 1 , . . . , an) is hamiltonian. (A sequence pointwise

( d 1 , . . . , dn) is  pointwise greater  than ( a 1 , . . . , an) if  di >  ai  for all  i.) greater

The following theorem characterizes all hamiltonian sequences:

Theorem 10.2.1. (Chvátal 1972)

An integer sequence ( a 1 , . . . , an)  such that  0 6  a 1 6  . . .  6  an < n and n > 3  is hamiltonian if and only if the following holds for every i < n/ 2 : ai  6  i ⇒ an−i >  n − i .

( a 1 , . . . , an)  Proof .

Let ( a 1 , . . . , an) be an arbitrary integer sequence such that 0 6  a 1 6  . . .  6  an < n  and  n > 3. We first assume that this sequence satisfies the condition of the theorem and prove that it is hamiltonian.

Suppose not; then there exists a graph  G = ( V, E) with a degree sequence ( d 1 , . . . , dn) ( d 1 , . . . , dn) such that di >  ai

for all  i

(1)

G = ( V, E)

but  G  has no Hamilton cycle. Let  G  be chosen with the maximum num-v 1 , . . . , vn

ber of edges, and let ( v 1 , . . . , vn) be an enumeration of  V  with  d( vi) =  di for all  i. By (1), our assumptions for ( a 1 , . . . , an) transfer to ( d 1 , . . . , dn), i.e.,

di  6  i ⇒ dn−i >  n − i

for all  i < n/ 2.

(2)

x, y

Let  x, y  be distinct and non-adjacent vertices in  G, with  d( x) 6  d( y) and  d( x) +  d( y) as large as possible. One easily checks that the degree sequence of  G +  xy  is pointwise greater than ( d 1 , . . . , dn), and hence than ( a 1 , . . . , an). Hence, by the maximality of  G, the new edge  xy  lies on a x 1 , . . . , xn

Hamilton cycle  H  of  G +  xy. Then  H − xy  is a Hamilton path  x 1 , . . . , xn in  G, with  x 1 =  x  and  xn =  y  say.

As in the proof of Dirac’s theorem, we now consider the index sets I :=  { i | xxi+1  ∈ E }  and  J :=  { j | xjy ∈ E } .

10.2 Hamilton cycles and degree sequences

217

Then  I ∪ J ⊆ {  1 , . . . , n −  1  }, and  I ∩ J =  ∅  because  G  has no Hamilton cycle. Hence

d( x) +  d( y) =  |I| +  |J| < n , (3)

so  h :=  d( x)  < n/ 2 by the choice of  x.

h

Since  xiy /

∈ E  for all  i ∈ I, all these  xi  were candidates for the choice of  x (together with  y). Our choice of  { x, y }  with  d( x) +  d( y) maximum thus implies that  d( xi) 6  d( x) for all  i ∈ I. Hence  G  has at least  |I| =  h  vertices of degree at most  h, so  dh  6  h. By (2), this implies that  dn−h >  n − h, i.e. the  h + 1 vertices  vn−h, . . . , vn  all have degree at least  n − h. Since  d( x) =  h, one of these vertices,  z  say, is not adjacent z

to  x. Since

d( x) +  d( z) >  h + ( n − h) =  n , this contradicts the choice of  x  and  y  by (3).

Let us now show that, conversely, for every sequence ( a 1 , . . . , an) of the theorem with

ah  6  h  and  an−h  6  n − h −  1

for some  h < n/ 2, there exists a graph that has a pointwise greater degree h

sequence than ( a 1 , . . . , an) but no Hamilton cycle. Clearly it suffices, given  h, to show this for the greatest such sequence ( a 1 , . . . , an), the sequence

( h, . . . , h

| {z }  , n − h −  1 , . . . , n − h −  1

|

{z

}  , n −  1 , . . . , n −  1

|

{z

})  .

(4)

h  times

n− 2 h  times

h  times

vh

vn

...

...

Kn−h

v 2

v 1

vn−h+1

Kh,h

...

vh+1

Fig. 10.2.1.  Any cycle containing  v 1 , . . . , vh  misses  vh+1

As Figure 10.2.1 shows, there is indeed a graph with degree sequence

(4) but no Hamilton cycle: the graph with vertices  v 1 , . . . , vn  and edge set

{ vivj | i, j > h } ∪ { vivj | i  6  h;  j > n − h } ,

218

10. Hamilton Cycles

i.e. the union of a  Kn−h  on the vertices  vh+1 , . . . , vn  and a  Kh,h  with partition sets  { v 1 , . . . , vh }  and  { vn−h+1 , . . . , vn }.

¤

By applying Theorem 10.2.1 to  G ∗ K 1, one can easily prove the following adaptation of the theorem to Hamilton paths. Let an integer sequence be called  path-hamiltonian  if every graph with a pointwise greater degree sequence has a Hamilton path.

Corollary 10.2.2.  An integer sequence ( a 1 , . . . , an)  such that n > 2  and 0 6  a 1 6  . . .  6  an < n is path-hamiltonian if and only if every i  6  n/ 2

is such that ai < i ⇒ an+1 −i >  n − i.

¤

10.3 Hamilton cycles in the square of a graph

Gd

Given a graph  G  and a positive integer  d, we denote by  Gd  the graph on V ( G) in which two vertices are adjacent if and only if they have distance at most  d  in  G. Clearly,  G =  G 1  ⊆ G 2  ⊆ . . .  Our goal in this section is to prove the following fundamental result:

Theorem 10.3.1. (Fleischner 1974)

If G is a 2-connected graph, then G 2  has a Hamilton cycle.

We begin with three simple lemmas. Let us say that an edge  e ∈ G 2

bridges

bridges  a vertex  v ∈ G  if its ends are neighbours of  v  in  G.

Lemma 10.3.2.  Let P =  v 0  . . . vk be a path (k > 1 ), and let G be the graph obtained from P by adding two vertices u, w, together with the edges uv 1  and wvk (Fig. 10.3.1).

(i)  P  2  contains a path Q from v 0  to v 1  with V ( Q) =  V ( P )  and vk− 1 vk ∈ E( Q) , such that each of the vertices v 1 , . . . , vk− 1  is bridged by an edge of Q.

(ii)  G 2  contains disjoint paths Q from v 0  to vk and Q0 from u to w, such that V ( Q)  ∪ V ( Q0) =  V ( G)  and each of the vertices v 1 , . . . , vk is bridged by an edge of Q or Q0.

u

w

P

v 0

v

vk

1

Fig. 10.3.1.  The graph  G  in Lemma 10.3.2

10.3 Hamilton cycles in the square of a graph

219

Proof . (i) If  k  is even, let  Q :=  v 0 v 2  . . . vk− 2 vkvk− 1 vk− 3  . . . v 3 v 1. If  k  is odd, let  Q :=  v 0 v 2  . . . vk− 1 vkvk− 2  . . . v 3 v 1.

(ii) If  k  is even, let  Q :=  v 0 v 2  . . . vk− 2 vk; if  k  is odd, let  Q :=

v 0 v 1 v 3  . . . vk− 2 vk. In both cases, let  Q0  be the  u– w  path on the remaining vertices of  G 2.

¤

Lemma 10.3.3.  Let G = ( V, E)  be a cubic multigraph with a Hamilton cycle C. Let e ∈ E( C)  and f ∈ E  r  E( C)  be edges with a common end v (Fig. 10.3.2). Then there exists a closed walk in G that traverses e once, every other edge of C once or twice, and every edge in E  r  E( C)  once.

This walk can be chosen to contain the triple ( e, v, f ) , that is, it traverses e in the direction of v and then leaves v by the edge f .

e

e

v

v0

v00

f

f

G

G0

Fig. 10.3.2.  The multigraphs  G  and  G0  in Lemma 10.3.3

Proof . By Proposition 1.2.1,  C  has even length. Replace every other

(1.2.1)

(1.8.1)

edge of  C  by a double edge, in such a way that  e  does not get replaced.

In the arising 4-regular multigraph  G0, split  v  into two vertices  v0, v00, making  v0  incident with  e  and  f , and  v00  incident with the other two edges at  v (Fig. 10.3.2). By Theorem 1.8.1 this multigraph has an Euler tour, which induces the desired walk in  G.

¤

Lemma 10.3.4.  For every 2-connected graph G and x ∈ V ( G) , there is a cycle C ⊆ G that contains x as well as a vertex y 6=  x with NG( y)  ⊆ V ( C) .

Proof . If  G  has a Hamilton cycle, there is nothing more to show. If not, let  C0 ⊆ G  be any cycle containing  x; such a cycle exists, since  G

is 2-connected. Let  D  be a component of  G − C0. Assume that  C0  and D  are chosen so that  |D|  is minimal. Since  G  is 2-connected,  D  has at least two neighbours on  C0. Then  C0  contains a path  P  between two such neighbours  u  and  v, whose interior ˚

P  does not contain  x  and

has no neighbour in  D (Fig. 10.3.3). Replacing  P  in  C0  by a  u– v  path through  D, we obtain a cycle  C  that contains  x  and a vertex  y ∈ D. If y  had a neighbour  z  in  G − C, then  z  would lie in a component  D0 $  D

of  G − C, contradicting the choice of  C0  and  D. Hence all the neighbours of  y  lie on  C, and  C  satisfies the assertion of the lemma.

¤

220

10. Hamilton Cycles

D

u

P

x

v

y

C

Fig. 10.3.3.  The proof of Lemma 10.3.4

Proof of Theorem 10.3.1. We show by induction on  |G|  that, given any vertex  x∗ ∈ G, there is a Hamilton cycle  H  in  G 2 with the following property:

Both edges of H at x∗ lie in G.

( ∗)

For  |G| = 3, we have  G =  K 3 and the assertion is trivial. So let  |G| > 4, x∗

assume the assertion for graphs of smaller order, and let  x∗ ∈ V ( G) be given. By Lemma 10.3.4, there is a cycle  C ⊆ G  that contains both  x∗

C

y∗

and a vertex  y∗ 6=  x∗  whose neighbours in  G  all lie on  C.

If  C  is a Hamilton cycle of  G, there is nothing to show; so assume that  G − C 6=  ∅. Consider a component  D  of  G − C. Let ˜

D  denote

the graph  G/( G − D) obtained from  G  by contracting  G − D  into a new P( D)

vertex ˜

x. If  |D| = 1, set  P( D) :=  { D }. If  |D| >  1, then ˜

D  is again

2-connected. Hence, by the induction hypothesis, ˜

D 2 has a Hamilton

˜

C

cycle ˜

C  whose edges at ˜

x  both lie in ˜

D. Note that the path ˜

C − ˜

x  may

have some edges that do not lie in  G 2: edges joining two neighbours of ˜

x

that have no common neighbour in  G (and are themselves non-adjacent P( D)

in  G). Let ˜

E  denote the set of these edges, and let  P( D) denote the set of components of ( ˜

C − ˜

x)  − ˜

E; this is a set of paths in  G 2 whose ends

are adjacent to ˜

x  in ˜

D (Fig. 10.3.4).

P( D)

D

˜

x

Fig. 10.3.4. P( D) consists of three paths, one of which is trivial P

Let  P  denote the union of the sets  P( D) over all components  D

of  G − C. Clearly,  P  has the following properties:

10.3 Hamilton cycles in the square of a graph

221

The elements of P are pairwise disjoint paths in G 2  avoid-

S

ing C, and V ( G) =  V ( C)  ∪ P ∈P V ( P ) . Every end y of a (1)

path P ∈ P has a neighbour on C in G;  we choose such a

neighbour and call it the  foot  of  P  at  y.

foot

If  P ∈ P  is trivial, then  P  has exactly one foot. If  P  is non-trivial, then P  has a foot at each of its ends. These two feet need not be distinct, however; so any non-trivial  P  has either one or two feet.

We shall now modify  P  a little, preserving the properties summa-rized under (1); no properties of  P  other than those will be used later in the proof. If a vertex of  C  is a foot of two distinct paths  P, P 0 ∈ P, say at  y ∈ P  and at  y0 ∈ P 0, then  yy0  is an edge and  P yy0P 0  is a path in  G 2; we replace  P  and  P 0  in  P  by this path. We repeat this modification of P  until the following holds:

No vertex of C is a foot of two distinct paths in P.

(2)

For  i = 1 ,  2 let  Pi ⊆ P  denote the set of all paths in  P  with exactly  i P 1 , P 2

feet, and let  Xi ⊆ V ( C) denote the set of all feet of paths in  Pi. Then X 1 , X 2

X 1  ∩ X 2 =  ∅  by (2), and  y∗ /

∈ X 1  ∪ X 2.

Let us also simplify  G  a little; again, these changes will affect neither the paths in  P  nor the validity of (1) and (2). First, we shall assume from now on that all elements of  P  are paths in  G  itself, not just in  G 2. This assumption may give us some additional edges for  G 2, but we shall not use these in our construction of the desired Hamilton cycle  H. (Indeed, H  will contain all the paths from  P  whole, as subpaths.) Thus if  H  lies in  G 2 and satisfies ( ∗) for the modified version of  G, it will do so also for the original. For every  P ∈ P, we further delete all  P – C  edges in  G

except those between the ends of  P  and its corresponding feet. Finally, we delete all chords of  C  in  G. We are thus assuming without loss of generality:

The only edges of G between C and a path P ∈ P are

the two edges between the ends of P and its corresponding

(3)

feet. (If |P | = 1 , these two edges coincide.) The only edges of G with both ends on C are the edges of C itself.

Our goal is to construct the desired Hamilton cycle  H  of  G 2 from the paths in  P  and suitable paths in  C 2. As a first approximation, we shall construct a closed walk  W  in the graph

[

˜

G :=  G −

P 1  ,

˜

G

a walk that will already satisfy a ( ∗)-type condition and traverse every path in  P 2 exactly once. Later, we shall modify  W  so that it passes through every vertex of  C  exactly once and, finally, so as to include the

222

10. Hamilton Cycles

paths from  P 1. For the construction of  W  we assume that  P 2  6=  ∅; the case of  P 2 =  ∅  is much simpler and will be treated later.

We start by choosing a fixed cyclic orientation of  C, a bijection i 7→ vi  from Z |C|  to  V ( C) with  vivi+1  ∈ E( C) for all  i ∈  Z |C|. Let us think of this orientation as clockwise; then every vertex  vi ∈ C  has a  right v+ , right

neighbour  v+ :=  v

:=  v

i

i+1 and a  left  neighbour  v−

i

i− 1. Accordingly, the

v−, left

edge  v−v  lies to the  left  of  v, the edge  vv+ lies on its  right, and so on.

A non-trivial path  P =  vivi+1  . . . vj− 1 vj  in  C  such that  V ( P )  ∩ X 2 =

interval

{ vi, vj }  will be called an  interval, with  left end vi  and  right end vj.

Thus,  C  is the union of  |X 2 | = 2  |P 2 |  intervals. As usual, we write  P =:

[  v, w ]  etc.

[  vi, vj ] and set ( vi, vj) := ˚

P  as well as [  vi, vj) :=  P˚

vj  and ( vi, vj ] := ˚

viP .

For intervals [  u, v ] and [  v, w ] with a common end  v  we say that [  u, v ]

lies to the  left  of [  v, w ], and [  v, w ] lies to the  right  of [  u, v ]. We denote I∗, P ∗

the unique interval [  v, w ] with  x∗ ∈ ( v, w ] as  I∗, the path in  P 2 with Q∗

foot  w  as  P ∗, and the path  I∗wP ∗  as  Q∗.

For the construction of  W , we may think of ˜

G  as a multigraph  M

on  X 2 whose edges are the intervals on  C  and the paths in  P 2 (with their feet as ends). By (2),  M  is cubic, so we may apply Lemma 10.3.3 with W

e :=  I∗  and  f :=  P ∗. The lemma provides us with a closed walk  W  in ˜

G

which traverses  I∗  once, every other interval of  C  once or twice, and every path in  P 2 once. Moreover,  W  contains  Q∗  as a subpath. The two edges at  x∗  of this path lie in  G; in this sense,  W  already satisfies ( ∗).

Let us now modify  W  so that  W  passes through every vertex of  C

exactly once. Simultaneously, we shall prepare for the later inclusion of e( v)

the paths from  P 1 by defining a map  v 7→ e( v) that is injective on  X 1

and assigns to every  v ∈ X 1 an edge  e( v) of the modified  W  with the following property:

The edge e( v)  either bridges v or is incident with it. In the ( ∗∗)

latter case, e( v)  ∈ C and e( v)  6=  vx∗.

For simplicity, we shall define the map  v 7→ e( v) on all of  V ( C) r  X 2, a set that includes  X 1 by (2). To ensure injectivity on  X 1, we only have to make sure that no edge  vw ∈ C  is chosen both as  e( v) and as  e( w). Indeed, since  |X 1 | > 2 if injectivity is a problem, and  P 2  6=  ∅

by assumption, we have  |C − y∗| >  |X 1 | + 2  |P 2 | > 4 and hence  |C| > 5; thus, no edge of  G 2 can bridge more than one vertex of  C, or bridge a vertex of  C  and lie on  C  at the same time.

For our intended adjustments of  W  at the vertices of  C, we consider the intervals of  C  one at a time. By definition of  W , every interval is of one of the following three types:

Type 1 :  W  traverses  I  once;

Type 2 :  W  traverses  I  twice, in one direction and back immediately afterwards (formally:  W  contains a triple ( e, x, e) with  x ∈ X 2

and  e ∈ E( I));

10.3 Hamilton cycles in the square of a graph

223

Type 3 :  W  traverses  I  twice, on separate occasions (i.e., there is no triple as above).

By definition of  W , the interval  I∗  is of type 1. The vertex  x  in the definition of a type 2 interval will be called the  dead end  of that interval.

dead end

Finally, since  Q∗  is a subpath of  W  and  W  traverses both  I∗  and  P ∗

only once, we have:

The interval to the right of I∗ is of type 2 and has its dead

(4)

end on the left.

Consider a fixed interval  I = [  x 1 , x 2 ]. Let  y 1 be the neighbour I, x 1 , x 2

of  x 1, and  y 2 the neighbour of  x 2 on a path in  P 2. Let  I−  denote the y 1 , y 2

interval to the left of  I.

I−

Suppose first that  I  is of type 1. We then leave  W  unchanged on  I.

If  I 6=  I∗  we choose as  e( v), for each  v ∈ ˚

I, the edge to the left of  v. As

I− 6=  I∗  by (4), and hence  x 1  6=  x∗, these choices of  e( v) satisfy ( ∗∗). If I =  I∗, we define  e( v) as the edge left of  v  if  v ∈ ( x 1 , x∗ ]  ∩ ˚

I, and as the

edge right of  v  if  v ∈ ( x∗, x 2). These choices of  e( v) are again compatible with ( ∗∗).

Suppose now that  I  is of type 2. Assume first that  x 2 is the dead end of  I. Then  W  contains the walk  y 1 x 1 Ix 2 Ix 1 I− (possibly in reverse order). We now apply Lemma 10.3.2 (i) with  P :=  y 1 x 1 I˚

x 2, and replace

in  W  the subwalk  y 1 x 1 Ix 2 Ix 1 by the  y 1– x 1 path  Q ⊆ G 2 of the lemma (Fig. 10.3.5). Then  V ( ˚

Q) =  V ( P ) r  { y 1 , x 1  } =  V (˚

I). The vertices

y 1

y 1

Q

W

x

x 2

x

I−

1

e( x−)

x

1

I

2

2

Fig. 10.3.5.  How to modify  W  on an interval of type 2

v ∈ ( x 1 , x−) are each bridged by an edge of  Q, which we choose as  e( v).

2

As  e( x−) we choose the edge to the left of  x− (unless  x− =  x 2

2

2

1). This

edge, too, lies on  Q, by the lemma. Moreover, by (4) it is not incident with  x∗ (since  x 2 is the dead end of  I, by assumption) and hence satisfies ( ∗∗). The case that  x 1 is the dead end of  I  can be treated in the same way: using Lemma 10.3.2 (i), we replace in  W  the subwalk y 2 x 2 Ix 1 Ix 2 by a  y 2– x 2 path  Q ⊆ G 2 with  V (˚

Q) =  V (˚

I), choose as  e( v)

for  v ∈ ( x+ , x

) as the edge

1

2) an edge of  Q  bridging  v, and define  e( x+

1

to the right of  x+ (unless  x+ =  x

1

1

2).

Suppose finally that  I  is of type 3. Since  W  traverses the edge  y 1 x 1

only once and the interval  I−  no more than twice,  W  contains  y 1 x 1 I and  I− ∪ I  as subpaths, and  I−  is of type 1. By (4), however,  I− 6=  I∗.

Hence, when  e( v) was defined for the vertices  v ∈ ˚

I−, the rightmost edge

x−x

1

1 of  I −  was not chosen as  e( v) for any  v, so we may now replace this

224

10. Hamilton Cycles

edge. Since  W  traverses  I+ no more than twice, it must traverse the edge  x 2 y 2 immediately after one of its two subpaths  y 1 x 1 I  and  x−x 1

1 I .

Take the starting vertex of this subpath ( y 1 or  x−) as the vertex  u  in 1

Lemma 10.3.2 (ii), and the other vertex in  { y 1 , x− }  as  v 1

0; moreover, set

vk :=  x 2 and  w :=  y 2. Then the lemma enables us to replace these two subpaths of  W  between  { y 1 , x− }  and  { x 1

2 , y 2  }  by disjoint paths in  G 2

(Fig. 10.3.6), and furthermore assigns to every vertex  v ∈ ˚

I  an edge  e( v)

of one of those paths, bridging  v.

y 1

y 2

y 1

y 2

I

v 0  x

x 2 =  vk

x 1

x 2

1

Fig. 10.3.6.  A type 3 modification for the case  u =  y 1 and  k  odd Following the above modifications,  W  is now a closed walk in ˜

G 2.

Let us check that, moreover,  W  contains every vertex of ˜

G  exactly once.

For vertices of the paths in  P 2 this is clear, because  W  still traverses every such path once and avoids it otherwise. For the vertices of  C − X 2, it follows from the above modifications by Lemma 10.3.2. So how about the vertices in  X 2?

Let  x ∈ X 2 be given, and let  y  be its neighbour on a path in  P 2. Let I 1 denote the interval  I  that satisfied  yxI ⊆ W  before the modification of  W , and let  I 2 denote the other interval ending in  x. If  I 1 is of type 1, then  I 2 is of type 2 with dead end  x. In this case,  x  was retained in  W

when  W  was modified on  I 1 but skipped when  W  was modified on  I 2, and is thus contained exactly once in  W  now. If  I 1 is of type 2, then  x is not its dead end, and  I 2 is of type 1. The subwalk of  W  that started with  yx  and then went along  I 1 and back, was replaced with a  y– x  path.

This path is now followed on  W  by the unchanged interval  I 2, so in this case too the vertex  x  is now contained in  W  exactly once. Finally, if  I 1

is of type 3, then  x  was contained in one of the replacement paths  Q, Q0

from Lemma 10.3.2 (ii); as these paths were disjoint by the assertion of the lemma,  x  is once more left on  W  exactly once.

We have thus shown that  W , after the modifications, is a closed walk in ˜

G 2 containing every vertex of ˜

G  exactly once, so  W  defines a Hamilton

˜

H

cycle ˜

H  of ˜

G 2. Since  W  still contains the path  Q∗, ˜

H  satisfies ( ∗).

Up until now, we have assumed that  P 2 is non-empty. If  P 2 =  ∅,

˜

H

let us set ˜

H := ˜

G =  C; then, again, ˜

H  satisfies ( ∗). It remains to turn

˜

H  into a Hamilton cycle  H  of  G 2 by incorporating the paths from  P 1.

In order to be able to treat the case of  P 2 =  ∅  along with the case of P 2  6=  ∅, we define a map  v 7→ e( v) also when  P 2 =  ∅, as follows: for

10.3 Hamilton cycles in the square of a graph

225

every  v ∈ C − y∗, set

½  vv+ if  v ∈ [ x∗,y∗)

e( v) :=

vv−

if  v ∈ ( y∗, x∗).

(Here, [  x∗, y∗) and ( y∗, x∗) denote the obvious paths in  C  defined analogously to intervals.) As before, this map  v 7→ e( v) is injective, satisfies ( ∗∗), and is defined on a superset of  X 1; recall that  y∗  cannot lie in  X 1 by definition.

Let  P ∈ P 1 be a path to be incorporated into ˜

H, say with foot

P, v

v ∈ X 1 and ends  y 1 , y 2. (If  |P | = 1, then  y 1 =  y 2.) Our aim is to replace y 1 , y 2

the edge  e :=  e( v) in ˜

H  by  P ; we thus have to show that the ends of  P

e

are joined to those of  e  by suitable edges of  G 2.

By (2) and (3),  v  has only two neighbours in ˜

G, its neighbours

x 1 , x 2 on  C. If  v  is incident with  e, i.e. if  e =  vxi  with  i ∈ {  1 ,  2  }, we replace  e  by the path  vy 1 P y 2 xi ⊆ G 2 (Fig. 10.3.7). If  v  is not incident y 1

P

y 2

v

x

e

1

x 2

C

Fig. 10.3.7.  Replacing the edge  e  in ˜

H

with  e  then  e  bridges  v, by ( ∗∗). Then  e =  x 1 x 2, and we replace  e by the path  x 1 y 1 P y 2 x 2  ⊆ G 2 (Fig. 10.3.8). Since  v 7→ e( v) is injective on  X 1, assertion (2) implies that all these modifications of ˜

H (one for

every  P ∈ P 1) can be performed independently, and hence produce a Hamilton cycle  H  of  G 2.

H

y 1

P

y 2

v

x 1

x 2

C

e

Fig. 10.3.8.  Replacing the edge  e  in ˜

H

Let us finally check that  H  satisfies ( ∗), i.e. that both edges of  H

at  x∗  lie in  G. Since ( ∗) holds for ˜

H, it suffices to show that any edge

e =  x∗z  of ˜

H  that is not in  H (and hence has the form  e =  e( v) for some e, z

v ∈ X 1) was replaced by an  x∗– z  path whose first edge lies in  G.

v

Where can the vertex  v  lie? Let us show that  v  must be incident with  e. If not then  P 2  6=  ∅, and  e  bridges  v. Now  P 2  6=  ∅  and  v ∈ X 1

together imply that  |C − y∗| >  |X 1 | + 2  |P 2 | > 3, so  |C| > 4. As  e ∈ G

(by ( ∗) for ˜

H), the fact that  e  bridges  v  thus contradicts (3).

226

10. Hamilton Cycles

So  v  is indeed incident with  e. Hence  v ∈ { x∗, z }  by definition of  e, while  e 6=  vx∗  by ( ∗∗). Thus  v =  x∗, and  e  was replaced by a path of the form  x∗y 1 P y 2 z. Since  x∗y 1 is an edge of  G, this replacement again preserves ( ∗). Therefore  H  does indeed satisfy ( ∗), and our induction is complete.

¤

We close the chapter with a far-reaching conjecture generalizing

Dirac’s theorem:

Conjecture. (Seymour 1974)

Let G be a graph of order n > 3 , and let k be a positive integer. If G

has minimum degree

δ( G) >

k

n ,

k + 1

then G has a Hamilton cycle H such that Hk ⊆ G.

For  k = 1, this is precisely Dirac’s theorem. The case  k = 2 had already been conjectured by Pósa in 1963 and was proved for large  n  by Komlós, Sárközy & Szemerédi (1996).

Exercises

1.

Show that every uniquely 3-edge-colourable cubic graph is hamilton-

ian. (‘Unique’ means that all 3-edge-colourings induce the same edge

partition.)

2.

Prove or disprove the following strengthening of Proposition 10.1.2:

‘Every  k-connected graph  G  with  |G| > 3 and  χ( G) >  |G|/k  has a Hamilton cycle.’

3.

Given a graph  G, consider a maximal sequence  G 0 , . . . , Gk  such that G 0 =  G  and  Gi+1 =  Gi +  xiyi  for  i = 0 , . . . , k −  1, where  xi, yi  are two non-adjacent vertices of  Gi  satisfying  dG ( x

( y

i

i) +  dGi

i) >  |G|. The last

graph of the sequence,  Gk, is called the  Hamilton closure  of  G. Show that this graph depends only on  G, not on the choice of the sequence G 0 , . . . , Gk.

4.

Let  x, y  be two nonadjacent vertices of a connected graph  G, with d( x) +  d( y) >  |G|. Show that  G  has a Hamilton cycle if and only if  G +  xy  has one. Using the previous exercise, deduce the following strengthening of Dirac’s theorem: if  d( x) +  d( y) >  |G|  for every two non-adjacent vertices  x, y ∈ G, then  G  has a Hamilton cycle.

5.

Given an even positive integer  k, construct for every  n >  k  a  k-regular graph of order 2 n + 1.

6.  −  Find a hamiltonian graph whose degree sequence is not hamiltonian.

Exercises

227

7.  −  Let  G  be a graph with fewer than  i  vertices of degree at most  i, for every  i < |G|/ 2. Use Chvátal’s theorem to show that  G  is hamiltonian.

(Thus in particular, Chv´

atal’s theorem implies Dirac’s theorem.)

8.

Find a connected graph  G  whose square  G 2 has no Hamilton cycle.

9.+ Show by induction on  |G|  that the third power  G 3 of a connected graph G  contains a Hamilton path between any two vertices. Deduce that  G 3

is hamiltonian.

10.

Show that the square of a 2-connected graph contains a Hamilton path

between any two vertices.

11.

An oriented complete graph is called a  tournament. Show that every tournament contains a (directed) Hamilton path.

12.+ Let  G  be a graph in which every vertex has odd degree. Show that every edge of  G  lies on an even number of Hamilton cycles.

(Hint. Let  xy ∈ E( G) be given. The Hamilton cycles through  xy correspond to the Hamilton paths in  G − xy  from  x  to  y. Consider the set  H  of  all  Hamilton paths in  G − xy  starting at  x, and show that an even number of these end in  y. To show this, define a graph on  H  so that the desired assertion follows from Proposition 1.2.1.)

Notes

The problem of finding a Hamilton cycle in a graph has the same kind of origin as its Euler tour counterpart and the four colour problem: all three problems come from mathematical puzzles older than graph theory itself. What began as a game invented by W.R. Hamilton in 1857—in which ‘Hamilton cycles’

had to be found on the graph of the dodecahedron—reemerged over a hun-

dred years later as a combinatorial optimization problem of prime importance: the  travelling salesman problem. Here, a salesman has to visit a number of customers, and his problem is to arrange these in a suitable circular route.

(For reasons not included in the mathematical brief, the route has to be such that after visiting a customer the salesman does not pass through that town again.) Much of the motivation for considering Hamilton cycles comes from variations of this algorithmic problem.

A detailed discussion of the various degree conditions for hamiltonicity referred to at the beginning of Section 10.2 can be found in R. Halin,  Graphentheorie, Wissenschaftliche Buchgesellschaft 1980. All the relevant references for Sections 10.1 and 10.2 can be found there, or in B. Bollobás,  Extremal Graph Theory, Academic Press 1978.

The ‘proof’ of the four colour theorem indicated at the end of Section 10.1, which is based on the (false) premise that every 3-connected cubic planar graph is hamiltonian, is usually attributed to the Scottish mathematician P.G. Tait.

Following Kempe’s flawed proof of 1879 (see the notes for Chapter 5), it seems that Tait believed to be in possession of at least one ‘new proof of Kempe’s theorem’. However, when he addressed the Edinburgh Mathematical Society on

228

10. Hamilton Cycles

this subject in 1883, he seems to have been aware that he could not—really—

prove the above statement about Hamilton cycles. His account in P.G. Tait, Listing’s topologie,  Phil. Mag. 17 (1884), 30–46, makes some entertaining reading.

A shorter proof of Tutte’s theorem that 4-connected planar graphs are hamiltonian was given by C. Thomassen, A theorem on paths in planar graphs, J. Graph Theory 7 (1983), 169–176. Tutte’s counterexample to Tait’s assumption that even 3-connectedness suffices (at least for cubic graphs) is shown in Bollobás, and in J.A. Bondy & U.S.R. Murty,  Graph Theory with Applications, Macmillan 1976 (where Tait’s attempted proof is discussed in some detail).

Proposition 10.1.2 is due to Chvátal & Erd˝

os (1972). Our proof of Fleisch-

ner’s theorem is based on S. Ř´ıha, A new proof of the theorem by Fleischner,  J. Combin. Theory B 52 (1991), 117–123. Seymour’s conjecture is from P.D. Seymour, Problem 3, in (T.P. McDonough and V.C. Mavron, eds.)  Combinatorics, Cambridge University Press 1974. Pósa’s conjecture was proved for large  n  by J. Komlós, G.N. Sárközy & E. Szemerédi, On the square of a Hamiltonian cycle in dense graphs,  Random Structures and Algorithms 9 (1996), 193–211.

11

Random Graphs

At various points in this book, we already encountered the following

fundamental theorem of Erd˝

os:  for every integer k there is a graph

G with g( G)  > k and χ( G)  > k. In plain English: there exist graphs combining arbitrarily large girth with arbitrarily high chromatic number.

How could one prove such a theorem? The standard approach would

be to construct a graph with those two properties, possibly in steps

by induction on  k. However, this is anything but straightforward: the global nature of the second property forced by the first, namely, that

the graph should have high chromatic number ‘overall’ but be acyclic

(and hence 2-colourable) locally, flies in the face of any attempt to build it up, constructively, from smaller pieces that have the same or similar properties.

In his pioneering paper of 1959, Erd˝

os took a radically different

approach: for each  n  he defined a probability space on the set of graphs with  n  vertices, and showed that, for some carefully chosen probability measures, the probability that an  n-vertex graph has both of the above properties is positive for all large enough  n.

This approach, now called the  probabilistic method , has since unfold-ed into a sophisticated and versatile proof technique, in graph theory as much as in other branches of discrete mathematics. The theory of  random graphs  is now a subject in its own right. The aim of this chapter is to offer an elementary but rigorous introduction to random graphs:

no more than is necessary to understand its basic concepts, ideas and

techniques, but enough to give an inkling of the power and elegance

hidden behind the calculations.

Erd˝

os’s theorem asserts the existence of a graph with certain properties: it is a perfectly ordinary assertion showing no trace of the randomness employed in its proof. There are also results in random graphs

that are generically random even in their statement: these are theorems about  almost all  graphs, a notion we shall meet in Section 11.3. In the

230

11. Random Graphs

last section, we give a detailed proof of a theorem of Erd˝

os and Rényi

that illustrates a proof technique frequently used in random graphs, the so-called  second moment method .

11.1 The notion of a random graph

V

Let  V  be a fixed set of  n  elements, say  V =  {  0 , . . . , n −  1  }. Our aim is G

to turn the set  G  of all graphs on  V  into a probability space, and then to consider the kind of questions typically asked about random objects: What is the probability that a graph  G ∈ G  has this or that property?

What is the expected value of a given invariant on  G, say its expected girth or chromatic number?

Intuitively, we should be able to generate  G  randomly as follows.

For each  e ∈ [ V ]2 we decide by some random experiment whether or not e  shall be an edge of  G; these experiments are performed independently, and for each the probability of success—i.e. of accepting  e  as an edge p

for  G—is equal to some fixed1 number  p ∈ [ 0 ,  1 ]. Then if  G 0 is some fixed graph on  V , with  m  edges say, the elementary event  { G 0  }  has a q

probability of  pmq( n) −m

2

(where  q := 1  − p): with this probability, our

randomly generated graph  G  is this particular graph  G 0. (The probability that  G  is  isomorphic  to  G 0 will usually be greater.) But if the probabilities of all the elementary events are thus determined, then so is the entire probability measure of our desired space  G. Hence all that remains to be checked is that such a probability measure on  G, one for which all individual edges occur independently with probability  p, does indeed exist.2

In order to construct such a measure on  G  formally, we start by defining for every potential edge  e ∈ [ V ]2 its own little probability space Ω e

Ω e :=  {  0 e,  1 e }, choosing  Pe( {  1 e }) :=  p  and  Pe( {  0 e }) :=  q  as the Pe

probabilities of its two elementary events. As our desired probability

G( n, p)

space  G =  G( n, p) we then take the product space Y

Ω

Ω :=

Ω e .

e∈[ V ]2

1 Often, the value of  p  will depend on the cardinality  n  of the set  V  on which our random graphs are generated; thus,  p  will be the value  p =  p( n) of some function n 7→ p( n). Note, however, that  V (and hence  n) is fixed for the definition of  G: for each  n  separately, we are constructing a probability space of the graphs  G  on V =  {  0 , . . . , n −  1  }, and within each space the probability that  e ∈ [ V ]2 is an edge of  G  has the same value for all  e.

2 Any reader ready to believe this may skip ahead now to the end of Proposition 11.1.1, without missing anything.

11.1 The notion of a random graph

231

Thus, formally, an element of Ω is a map  ω  assigning to every  e ∈ [ V ]2

either 0 e  or 1 e, and the probability measure  P  on Ω is the product P

measure of all the measures  Pe. In practice, of course, we identify  ω

with the graph  G  on  V  whose edge set is

E( G) =  { e | ω( e) = 1 e } ,

and call  G  a  random graph  on  V  with edge probability  p.

random

graph

Following standard probabilistic terminology, we may now call any

set of graphs on  V  an  event  in  G( n, p). In particular, for every  e ∈ [ V ]2

event

the set

Ae :=  { ω | ω( e) = 1 e }

Ae

of all graphs  G  on  V  with  e ∈ E( G) is an event: the event that  e  is an edge of  G. For these events, we can now prove formally what had been our guiding intuition all along:

Proposition 11.1.1.  The events Ae are independent and occur with probability p.

Proof . By definition,

Y

Ae =  {  1 e } ×

Ω e0 .

e06= e

Since  P  is the product measure of all the measures  Pe, this implies Y

P ( Ae) =  p ·

1 =  p .

e06= e

Similarly, if  { e 1 , . . . , ek }  is any subset of [ V ]2, then ³

Y

´

P ( Ae ∩ . . . ∩ A ) =  P {  1  } × . . . × {  1  } ×

Ω

1

ek

e 1

ek

e

e /

∈{ e 1 ,...,ek }

=  pk

=  P ( Ae )  · · · P ( A )  .

1

ek

¤

As noted before,  P  is determined uniquely by the value of  p  and our assumption that the events  Ae  are independent. In order to calculate probabilities in  G( n, p), it therefore generally suffices to work with these two assumptions: our concrete model for  G( n, p) has served its purpose and will not be needed again.

As a simple example of such a calculation, consider the event that  G

contains some fixed graph  H  on a subset of  V  as a subgraph; let  |H| =:  k k

and  kHk =:  `. The probability of this event  H ⊆ G  is the product of

`

the probabilities  Ae  over all the edges  e ∈ H, so  P [  H ⊆ G ] =  p`. In

232

11. Random Graphs

contrast, the probability that  H  is an  induced  subgraph of  G  is  p`q( k) −`

2

:

now the edges missing from  H  are required to be missing from  G  too, and they do so independently with probability  q.

The probability  PH  that  G  has an induced subgraph  isomorphic to  H  is usually more difficult to compute: since the possible instances of  H  on subsets of  V  overlap, the events that they occur in  G  are not independent. However, the sum (over all  k-sets  U ⊆ V ) of the probabilities  P [  H ' G [  U ] ] is always an upper bound for  PH, since  PH  is the measure of the union of all those events. For example, if  H =  Kk, we have the following trivial upper bound on the probability that  G  contains an induced copy of  H:

[ 11.2.1 ]

Lemma 11.1.2.  For all integers n, k with n >  k > 2 , the probability

[ 11.3.4 ]

that G ∈ G( n, p)  has a set of k independent vertices is at most µ ¶

n

P [  α( G) >  k ] 6

q( k)

2  .

k

Proof . The probability that a fixed  k-set  U ⊆ V  is independent in G  is  q( k)

2 . The assertion thus follows from the fact that there are only

¡ ¢

n

such sets  U .

¤

k

Analogously, the probability that  G ∈ G( n, p) contains a  Kk  is at most

µ ¶

n

P [  ω( G) >  k ] 6

p( k)

2  .

k

Now if  k  is fixed, and  n  is small enough that these bounds for the probabilities  P [  α( G) >  k ] and  P [  ω( G) >  k ] sum to less than 1, then  G

contains graphs that have neither property: graphs which contain nei-

ther a  Kk  nor a  Kk  induced. But then any such  n  is a lower bound for the Ramsey number of  k !

As the following theorem shows, this lower bound is quite close to

the upper bound of 22 k− 3 implied by the proof of Theorem 9.1.1:

Theorem 11.1.3. (Erd˝

os 1947)

For every integer k > 3 , the Ramsey number of k satisfies R( k)  >  2 k/ 2 .

Proof . For  k = 3 we trivially have  R(3) > 3  >  23 / 2, so let  k > 4. We show that, for all  n  6 2 k/ 2 and  G ∈ G( n,  1 ), the probabilities  P [  α( G) >  k ]

2

and  P [  ω( G) >  k ] are both less than 1 .

2

Since  p =  q = 1 , Lemma 11.1.2 and the analogous assertion for  ω( G) 2

imply the following for all  n  6 2 k/ 2 (use that  k!  >  2 k  for  k > 4):

11.1 The notion of a random graph

233

µ ¶

n ¡ ¢( k)

P [  α( G) >  k ] , P [  ω( G) >  k ] 6

1

2

k

2

¡

¢

<

nk/ 2 k  2 −  1  k( k− 1)

2

¡

¢

6 2 k 2 / 2 / 2 k  2 − 1 k( k− 1) 2

= 2 −k/ 2

<  1  .

2

¤

In the context of random graphs, each of the familiar graph invari-

ants (like average degree, connectivity, girth, chromatic number, and so on) may be interpreted as a non-negative  random variable  on  G( n, p), random

variable

a function

X:  G( n, p)  → [ 0 , ∞)  .

The  mean  or  expected  value of  X  is the number mean

expectation

X

E( X) :=

P ( { G })  · X( G)  .

G∈G( n,p)

E( X)

Note that the operator  E, the  expectation, is linear: we have  E( X +  Y ) =

E( X) +  E( Y ) and  E( λX) =  λE( X) for any two random variables  X, Y

on  G( n, p) and  λ ∈  R.

Computing the mean of a random variable  X  can be a simple and

effective way to establish the existence of a graph  G  such that  X( G)  < a for some fixed  a >  0 and, moreover,  G  has some desired property  P.

Indeed, if the expected value of  X  is small, then  X( G) cannot be large for more than a few graphs in  G( n, p), because  X( G) > 0 for all  G ∈ G( n, p).

Hence  X  must be small for many graphs in  G( n, p), and it is reasonable to expect that among these we may find one with the desired property  P.

This simple idea lies at the heart of countless non-constructive exist-

ence proofs using random graphs, including the proof of Erd˝

os’s theorem

presented in the next section. Quantified, it takes the form of the following lemma, whose proof follows at once from the definition of the

expectation and the additivity of  P :

Lemma 11.1.4. (Markov’s Inequality)

[ 11.2.2 ]

Let X > 0  be a random variable on G( n, p)  and a >  0 . Then

[ 11.4.1 ]

[ 11.4.3 ]

P [  X >  a ] 6  E( X) /a .

Proof .

X

E( X) =

P ( { G })  · X( G)

G∈G( n,p)

234

11. Random Graphs

X

>

P ( { G })  · X( G)

G∈G( n,p)

X( G)>  a

X

>

P ( { G })  · a

G∈G( n,p)

X( G)>  a

=  P [  X >  a ]  · a .

¤

Since our probability spaces are finite, the expectation can often

be computed by a simple application of  double counting, a standard combinatorial technique we met before in the proofs of Corollary 4.2.8

and Theorem 5.5.3. For example, if  X  is a random variable on  G( n, p) that counts the number of subgraphs of  G  in some fixed set  H  of graphs on  V , then  E( X), by definition, counts the number of pairs ( G, H) such that  H ⊆ G, each weighted with the probability of  { G }. Algorithmically, we compute  E( X) by going through the graphs  G ∈ G( n, p) in an ‘outer loop’ and performing, for each  G, an ‘inner loop’ that runs through the graphs  H ∈ H  and counts ‘ P ( { G })’ whenever  H ⊆ G. Alternatively, we may count the same set of weighted pairs with  H  in the outer and G  in the inner loop: this amounts to adding up, over all  H ⊆ H, the probabilities  P [  H ⊆ G ].

To illustrate this once in detail, let us compute the expected number

of cycles of some given length  k > 3 in a random graph  G ∈ G( n, p). So X

let  X:  G( n, p)  →  N be the random variable that assigns to every random graph  G  its number of  k-cycles, the number of subgraphs isomorphic to  Ck. Let us write

( n) k

( n) k :=  n ( n −  1)( n −  2)  · · · ( n − k + 1) for the number of sequences of  k  distinct elements of a given  n-set.

[ 11.2.2 ]

Lemma 11.1.5.  The expected number of k-cycles in G ∈ G( n, p)  is

[ 11.4.3 ]

( n)

E( X) =

k pk.

2 k

Proof . For every  k-cycle  C  with vertices in  V =  {  0 , . . . , n −  1  }, the vertex set of the graphs in  G( n, p), let  XC:  G( n, p)  → {  0 ,  1  }  denote the indicator random variable  of  C: n 1 if  C ⊆ G;

XC:  G 7→

0

otherwise.

Since  XC  takes only 1 as a positive value, its expectation  E( XC) equals the measure  P [  XC = 1 ] of the set of all graphs in  G( n, p) that contain  C.

But this is just the probability that  C ⊆ G:

E( XC) =  P [  C ⊆ G ] =  pk.

(1)

11.1 The notion of a random graph

235

How many such cycles  C =  v 0  . . . vk− 1 v 0 are there? There are ( n) k sequences  v 0  . . . vk− 1 of distinct vertices in  V , and each cycle is identified by 2 k  of those sequences—so there are exactly ( n) k/ 2 k  such cycles.

Our random variable  X  assigns to every graph  G  its number of  k-

cycles. Clearly, this is the sum of all the values  XC( G), where  C  varies over the ( n) k/ 2 k  cycles of length  k  with vertices in  V : X

X =

XC .

C

Since the expectation is linear, (1) thus implies

³ X

´

X

( n)

E( X) =  E

X

k

C

=

E( XC) =

pk

2 k

C

C

as claimed.

¤

11.2 The probabilistic method

Very roughly, the  probabilistic method  in discrete mathematics has developed from the following idea. In order to prove the existence of an

object with some desired property, one defines a probability space on

some larger—and certainly non-empty—class of objects, and then shows

that an element of this space has the desired property with positive

probability. The ‘objects’ inhabiting this probability space may be of

any kind: partitions or orderings of the vertices of some fixed graph arise as naturally as mappings, embeddings and, of course, graphs themselves.

In this section, we illustrate the probabilistic method by giving a detailed account of one of its earliest results: of Erd˝

os’s classic theorem on large

girth and chromatic number.

Erd˝

os’s theorem says that, given any positive integer  k, there is a graph  G  with girth  g( G)  > k  and chromatic number  χ( G)  > k. Let us call cycles of length at most  k short , and sets of  |G|/k  or more vertices short

big. For a proof of Erd˝

os’s theorem, it suffices to find a graph  G  without

big/small

short cycles and without big independent sets of vertices: then the colour classes in any vertex colouring of  G  are  small (not big), so we need more than  k  colours to colour  G.

How can we find such a graph  G? If we choose  p  small enough, then a random graph in  G( n, p) is unlikely to contain any (short) cycles. If we choose  p  large enough, then  G  is unlikely to have big independent vertex sets. So the question is: do these two ranges of  p  overlap, that is, can we choose  p  so that, for some  n, it is both small enough to give P [  g  6  k ]  <  1 and large enough for  P [  α >  n/k ]  <  1 ? If so, then 2

2

236

11. Random Graphs

G( n, p) will contain at least one graph without either short cycles or big independent sets.

Unfortunately, such a choice of  p  is impossible: the two ranges of  p do not overlap! As we shall see in Section 11.4, we must keep  p  below n− 1 to make the occurrence of short cycles in  G  unlikely—but for any such  p  there will most likely be no cycles in  G  at all (Exercise 19), so  G

will be bipartite and hence have at least  n/ 2 independent vertices.

But all is not lost. In order to make big independent sets unlikely,

we shall fix  p  above  n− 1, at  n²− 1 for some  ² >  0. Fortunately, though, if  ²  is small enough then this will produce only  few  short cycles in  G, even compared with  n (rather than, more typically, with  nk). If we then delete a vertex in each of those cycles, the graph  H  obtained will have no short cycles, and its independence number  α( H) will be at most that of  G. Since  H  is not much smaller than  G, its chromatic number will thus still be large, so we have found a graph with both large girth and large chromatic number.

To prepare for the formal proof of Erd˝

os’s theorem, we first show

that an edge probability of  p =  n²− 1 is indeed always large enough to ensure that  G ∈ G( n, p) ‘almost surely’ has no big independent set of vertices. More precisely, we prove the following slightly stronger assertion: Lemma 11.2.1.  Let k >  0  be an integer, and let p =  p( n)  be a function of n such that p > (6 k  ln  n) n− 1  for n large. Then lim  P [  α > 1  n/k ] = 0  .

n→∞

2

(11.1.2)

Proof . For all integers  n, r  with  n >  r > 2, and all  G ∈ G( n, p), Lemma

11.1.2 implies

µ ¶

n

P [  α >  r ] 6

q( r)

2

r

6  nrq( r)

2

³

´ r

=  nq( r− 1) / 2

³

´ r

6  ne−p( r− 1) / 2 ;

here, the last inequality follows from the fact that 1  − p  6  e−p  for all  p.

(Compare the functions  x 7→ ex  and  x 7→ x + 1 for  x =  −p.) Now if  p > (6 k  ln  n) n− 1 and  r > 1  n/k, then the term under the exponent 2

satisfies

ne−p( r− 1) / 2 =  ne−pr/ 2 +  p/ 2

6  ne−(3 / 2) ln  n +  p/ 2

6  n n− 3 / 2  e 1 / 2

√ √

=

e / n −→  0  .

n→∞

11.2 The probabilistic method

237

Since  p > (6 k  ln  n) n− 1 for  n  large, we thus obtain for  r :=  d  1  n/ke 2

lim  P [  α > 1  n/k ] = lim  P [  α >  r ] = 0  , n→∞

2

n→∞

as claimed.

¤

Theorem 11.2.2. (Erd˝

os 1959)

For every integer k there exists a graph H with girth g( H)  > k and

[ 9.2.3 ]

chromatic number χ( H)  > k.

(11.1.4)

Proof . Assume that  k > 3, fix  ²  with 0  < ² <  1 /k, and let  p :=  n²− 1. Let

(11.1.5)

p, ², X

X( G) denote the number of short cycles in a random graph  G ∈ G( n, p), i.e. its number of cycles of length at most  k.

By Lemma 11.1.5, we have

k

X

k

( n)

X

E( X) =

i pi  6 1

nipi  6 1 ( k −  2)  nkpk ;

2 i

2

2

i=3

i=3

note that ( np) i  6 ( np) k, because  np =  n² > 1. By Lemma 11.1.4,

±

P [  X >  n/ 2 ] 6  E( X) ( n/ 2) 6 ( k −  2)  nk− 1 pk

= ( k −  2)  nk− 1 n( ²− 1) k

= ( k −  2)  nk²− 1 .

As  k² −  1  <  0 by our choice of  ², this implies that lim  P [  X >  n/ 2 ] = 0  .

n→∞

Let  n  be large enough that  P [  X >  n/ 2 ]  <  1 and  P [  α > 1  n/k ]  <  1 ; n

2

2

2

the latter is possible by our choice of  p  and Lemma 11.2.1. Then there is a graph  G ∈ G( n, p) with fewer than  n/ 2 short cycles and  α( G)  < 1  n/k. From each of those cycles delete a vertex, and let  H  be the graph 2

obtained. Then  |H| >  n/ 2 and  H  has no short cycles, so  g( H)  > k. By definition of  G,

|H|

χ( H) >

>  n/ 2  > k .

α( H)

α( G)

¤

Corollary 11.2.3.  There are graphs with arbitrarily large girth and arbitrarily large values of the invariants κ, ε and δ.

Proof . Apply Corollary 5.2.3 and Theorem 1.4.2.

¤

(1.4.2)

(5.2.3)

238

11. Random Graphs

11.3 Properties of almost all graphs

property

A  graph property  is a class of graphs that is closed under isomorphism, one that contains with every graph  G  also the graphs isomorphic to  G.

If  p =  p( n) is a fixed function (possibly constant), and  P  is a graph property, we may ask how the probability  P [  G ∈ P ] behaves for  G ∈ G( n, p) as  n → ∞. If this probability tends to 1, we say that  G ∈ P  for  almost almost all

all (or  almost every)  G ∈ G( n, p), or that  G ∈ P almost surely; if it etc.

tends to 0, we say that  almost no G ∈ G( n, p) has the property  P. (For example, in Lemma 11.2.1 we proved that, for a certain  p, almost no G ∈ G( n, p) has a set of more than 1  n/k  independent vertices.) 2

To illustrate the new concept let us show that, for constant  p, every fixed abstract3 graph  H  is an induced subgraph of almost all graphs: Proposition 11.3.1.  For every constant p ∈ (0 ,  1)  and every graph H, almost every G ∈ G( n, p)  contains an induced copy of H.

Proof . Let  H  be given, and  k :=  |H|. If  n >  k  and  U ⊆ {  0 , . . . , n −  1  }

is a fixed set of  k  vertices of  G, then  G [  U ] is isomorphic to  H  with a certain probability  r >  0. This probability  r  depends on  p, but not on  n (why not?). Now  G  contains a collection of  bn/kc  disjoint such sets  U . The probability that none of the corresponding graphs  G [  U ] is isomorphic to  H  is (1  − r) bn/kc, since these events are independent by the disjointness of the edges sets [ U ]2. Thus

P [  H 6⊆ G  induced ] 6 (1  − r) bn/kc −→  0  , n→∞

which implies the assertion.

¤

The following lemma is a simple device enabling us to deduce that

quite a number of natural graph properties (including that of Proposi-

Pi,j

tion 11.3.1) are shared by almost all graphs. Given  i, j ∈  N, let  Pi,j denote the property that the graph considered contains, for any disjoint vertex sets  U, W  with  |U |  6  i  and  |W |  6  j, a vertex  v /

∈ U ∪ W  that is

adjacent to all the vertices in  U  but to none in  W .

Lemma 11.3.2.  For every constant p ∈ (0 ,  1)  and i, j ∈  N , almost every graph G ∈ G( n, p)  has the property Pi,j.

3 The word ‘abstract’ is used to indicate that only the isomorphism type of  H  is known or relevant, not its actual vertex and edge sets. In our context, it indicates that the word ‘subgraph’ is used in the usual sense of ‘isomorphic to a subgraph’.

11.3 Properties of almost all graphs

239

Proof . For fixed  U, W  and  v ∈ G − ( U ∪ W ), the probability that  v  is adjacent to all the vertices in  U  but to none in  W , is p|U|q|W | >  piqj.

Hence, the probability that no suitable  v  exists for these  U  and  W , is (1  − p|U|q|W |) n−|U|−|W |  6 (1  − piqj) n−i−j (for  n >  i +  j), since the corresponding events are independent for different  v. As there are no more than  ni+ j  pairs of such sets  U, W

in  V ( G) (encode sets  U  of fewer than  i  points as non-injective maps

{  0 , . . . , i −  1  } → {  0 , . . . , n −  1  }, etc.), the probability that some such pair has no suitable  v  is at most

ni+ j(1  − piqj) n−i−j,

which tends to zero as  n → ∞  since 1  − piqj <  1.

¤

Corollary 11.3.3.  For every constant p ∈ (0 ,  1)  and k ∈  N , almost every graph in G( n, p)  is k-connected.

Proof . By Lemma 11.3.2, it is enough to show that every graph in  P 2 ,k− 1

is  k-connected. But this is easy: any graph in  P 2 ,k− 1 has order at least k + 2, and if  W  is a set of fewer than  k  vertices, then by definition of P 2 ,k− 1 any other two vertices  x, y  have a common neighbour  v /∈ W ; in particular,  W  does not separate  x  from  y.

¤

In the proof of Corollary 11.3.3, we showed substantially more than

was asked for: rather than finding, for any two vertices  x, y /

∈ W , some

x– y  path avoiding  W , we showed that  x  and  y  have a common neighbour outside  W ; thus, all the paths needed to establish the desired connectivity could in fact be chosen of length 2. What seemed like a clever

trick in this particular proof is in fact indicative of a more fundamental phenomenon for constant edge probabilities: by an easy result in logic, any statement about graphs expressed by quantifying over vertices only

(rather than over sets or sequences of vertices)4 is either almost surely true or almost surely false. All such statements, or their negations,

are in fact immediate consequences of an assertion that the graph has

property  Pi,j, for some suitable  i, j.

As a last example of an ‘almost all’ result we now show that almost

every graph has a surprisingly high chromatic number:

4 In the terminology of logic: any first order sentence in the language of graph theory

240

11. Random Graphs

Proposition 11.3.4.  For every constant p ∈ (0 ,  1)  and every ² >  0 , almost every graph G ∈ G( n, p)  has chromatic number log(1 /q)

χ( G)  >

· n .

2 +  ²

log  n

(11.1.2)

Proof . For any fixed  n >  k > 2, Lemma 11.1.2 implies µ ¶

n

P [  α >  k ] 6

q( k)

2

k

6  nkq( k)

2

=  qk  log  n + 1  k( k− 1)

log  q

2

¡

¢

k

−  2 log  n

=  q

+ k− 1

2

log(1 /q)

.

For

log  n

k := (2 +  ²) log(1 /q)

the exponent of this expression tends to infinity with  n, so the expression itself tends to zero. Hence, almost every  G ∈ G( n, p) is such that in any vertex colouring of  G  no  k  vertices can have the same colour, so every colouring uses more than

n

log(1 /q)

=

· n

k

2 +  ²

log  n

colours.

¤

By a result of Bollobás (1988), Proposition 11.3.4 is sharp in the following sense: if we replace  ²  by  −², then the lower bound given for  χ

turns into an upper bound.

Most of the results of this section have the interesting common fea-

ture that the values of  p  played no role whatsoever: if almost every graph in  G( n,  1 ) had the property considered, then the same was true 2

for almost every graph in  G( n,  1 / 1000). How could this happen?

Such insensitivity of our random model to changes of  p  was certainly not intended: after all, among all the graphs with a certain property  P

it is often those having  P ‘only just’ that are the most interesting—for those graphs are most likely to have different properties too, properties to which  P  might thus be set in relation. (The proof of Erd˝os’s theorem

is a good example.) For most properties, however—and this explains the

above phenomenon—the critical order of magnitude of  p  around which the property will ‘just’ occur or not occur lies far below any constant value of  p: it is typically a function of  n  tending to zero as  n → ∞.

11.3 Properties of almost all graphs

241

Let us then see what happens if  p  is allowed to vary with  n. Almost immediately, a fascinating picture unfolds. For edge probabilities p  whose order of magnitude lies below  n− 2, a random graph  G ∈ G( n, p) almost surely has no edges at all. As  p  grows,  G  acquires more and

√

more structure: from about  p =

n n− 2 onwards, it almost surely has

a component with more than two vertices, these components grow into

trees, and around  p =  n− 1 the first cycles are born. Soon, some of these will have several crossing chords, making the graph non-planar. At the

same time, one component outgrows the others, until it devours them

around  p = (log  n) n− 1, making the graph connected. Hardly later, at p = (1 +  ²)(log  n) n− 1, our graph almost surely has a Hamilton cycle!

It has become customary to compare this development of random

graphs as  p  grows to the evolution of an organism: for each  p =  p( n), one thinks of the properties shared by almost all graphs in  G( n, p) as properties of ‘the’ typical random graph  G ∈ G( n, p), and studies how G  changes its features with the growth rate of  p. As with other species, the evolution of random graphs happens in relatively sudden jumps: the

critical edge probabilities mentioned above are thresholds below which

almost no graph and above which almost every graph has the property

considered. More precisely, we call a real function  t =  t( n) with  t( n)  6= 0

threshold

for all  n  a  threshold function  for a graph property  P  if the following function

holds for all  p =  p( n), and  G ∈ G( n, p): ½ 0 if  p/t→ 0 as  n→∞

lim  P [  G ∈ P ] =

n→∞

1

if  p/t → ∞  as  n → ∞.

If  P  has a threshold function  t, then clearly any positive multiple  ct  of  t is also a threshold function for  P; thus, threshold functions in the above sense are only ever unique up to a multiplicative constant.5

Which graph properties have threshold functions? Natural candi-

dates for such properties are  increasing  ones, properties closed under the addition of edges. (Graph properties of the form  { G | G ⊇ H }, with H  fixed, are common increasing properties; connectedness is another.) And indeed, Bollobás & Thomason (1987) have shown that all increasing properties, trivial exceptions aside, have threshold functions.

In the next section we shall study a general method to compute

threshold functions.

5 Our notion of threshold reflects only the crudest interesting level of screening: for some properties, such as connectedness, one can define sharper thresholds where the constant factor is crucial. Note also the role of the constant factor in our comparison of connectedness with hamiltonicity in the previous paragraph.

242

11. Random Graphs

11.4 Threshold functions and second moments

Consider a graph property of the form

P =  { G | X( G)  >  0  } ,

X > 0

where  X > 0 is a random variable on  G( n, p). Countless properties can be expressed naturally in this way; if  X  denotes the number of spanning trees, for example, then  P  corresponds to connectedness.

How could we prove that  P  has a threshold function  t? Any such proof will consist of two parts: a proof that almost no  G ∈ G( n, p) has P  when  p  is small compared with  t, and one showing that almost every G  has  P  when  p  is large.

If  X  is integral, we may use Markov’s inequality for the first part of the proof and find an upper bound for  E( X) instead of  P [  X >  0 ]: if  E( X) is small then  X( G) can be positive—and hence at least 1—only for few  G ∈ G( n, p). Besides, the expectation is much easier to calculate than probabilities: without worrying about such things as independence

or incompatibility of events, we may compute the expectation of a sum of random variables—for example, of indicator random variables—simply

by adding up their individual expected values.

For the second part of the proof, things are more complicated. In

order to show that  P [  X >  0 ] is large, it is not enough to bound  E( X) from below: since  X  is not bounded above,  E( X) may be large simply because  X  is very large on just a few graphs  G—so  X  may still be zero for most  G ∈ G( n, p).6 In order to prove that  P [  X >  0 ]  →  1, we thus have to show that this cannot happen, i.e. that  X  does not deviate a lot from its mean too often.

The following elementary tool from probability theory achieves just

that. As is customary, we write

µ

µ :=  E( X)

and define  σ > 0 by setting

¡

¢

σ 2

σ 2 :=  E ( X − µ)2  .

This quantity  σ 2 is called the  variance  or  second moment  of  X; by definition, it is a (quadratic) measure of how much  X  deviates from its mean. Since  E  is linear, the defining term for  σ 2 expands to σ 2 =  E( X 2  −  2 µX +  µ 2) =  E( X 2)  − µ 2 .

6 For some  p  between  n− 1 and (log  n) n− 1, for example, almost every  G ∈ G( n, p) has an isolated vertex (and hence no spanning tree), but its expected number of spanning trees tends to infinity with  n ! See the Exercise 13 for details.

11.4 Threshold functions and second moments

243

Note that  µ  and  σ 2 always refer to a random variable on some fixed probability space. In our setting, where we consider the spaces  G( n, p), both quantities are functions of  n.

The following lemma says exactly what we need: that  X  cannot

deviate a lot from its mean too often.

Lemma 11.4.1. (Chebyshev’s Inequality)

For all real λ >  0 ,

£

¤

P |X − µ| >  λ

6  σ 2 /λ 2 .

Proof . By Lemma 11.1.4 and definition of  σ 2,

(11.1.4)

P [  |X − µ| >  λ ] =  P [ ( X − µ)2 >  λ 2 ] 6  σ 2 /λ 2 .

¤

For a proof that  X( G)  >  0 for almost all  G ∈ G( n, p), Chebyshev’s inequality can be used as follows:

Lemma 11.4.2.  If µ >  0  for n large, and σ 2 /µ 2  →  0  as n → ∞, then X( G)  >  0  for almost all G ∈ G( n, p) .

Proof . Any graph  G  with  X( G) = 0 satisfies  |X( G)  − µ| =  µ. Hence

Lemma 11.4.1 implies with  λ :=  µ  that

£

¤

P [  X = 0 ] 6  P |X − µ| >  µ

6  σ 2 /µ 2  −→  0  .

n→∞

Since  X > 0, this means that  X >  0 almost surely, i.e. that  X( G)  >  0

for almost all  G ∈ G( n, p).

¤

As the main result of this section, we now prove a theorem that will

at once give us threshold functions for a number of natural properties.

Given a graph  H, we denote by  PH  the graph property of containing a PH

copy of  H  as a subgraph. We shall call  H balanced  if  ε( H0) 6  ε( H) for balanced

all subgraphs  H0  of  H.

Theorem 11.4.3. (Erd˝

os & Rényi 1960)

If H is a balanced graph with k vertices and ` > 1  edges, then t( n) :=

k, `

n−k/ìs a threshold function for PH.

t

244

11. Random Graphs

(11.1.4)

Proof . Let  X( G) denote the number of subgraphs of  G  isomorphic to  H.

(11.1.5)

Given  n ∈  N, let  H  denote the set of all graphs isomorphic to  H  whose X

vertices lie in  {  0 , . . . , n −  1  }, the vertex set of the graphs  G ∈ G( n, p):

©

ª

H

H :=  H0 | H0 ' H, V ( H0)  ⊆ {  0 , . . . , n −  1  } .

Given  H0 ∈ H  and  G ∈ G( n, p), we shall write  H0 ⊆ G  to express that H0  itself—not just an isomorphic copy of  H0—is a subgraph of  G.

h

By  h  we denote the number of isomorphic copies of  H  on a fixed

¡ ¢

k-set; clearly,  h  6  k! . As there are  n  possible vertex sets for the graphs k

in  H, we thus have

µ ¶

µ ¶

|H|

n

n

=

h  6

k! 6  nk.

(1)

k

k

p, γ

Given  p =  p( n), we set  γ :=  p/t; then p =  γn−k/`.

(2)

We have to show that almost no  G ∈ G( n, p) lies in  PH  if  γ →  0 as  n → ∞, and that almost all  G ∈ G( n, p) lie in  PH  if  γ → ∞  as  n → ∞.

For the first part of the proof, we find an upper bound for  E( X), the expected number of subgraphs of  G  isomorphic to  H. As in the proof of

Lemma 11.1.5, double counting gives

X

E( X) =

P [  H0 ⊆ G ]  .

(3)

H0 ∈H

For every fixed  H0 ∈ H, we have

P [  H0 ⊆ G ] =  p`,

(4)

because  kHk =  `. Hence,

E( X) =  |H| p`  6  nk( γn−k/`) ` =  γ`.

(5)

(3 ,  4)

(1 ,  2)

Thus if  γ →  0 as  n → ∞, then

P [  G ∈ PH ] =  P [  X > 1 ] 6  E( X) 6  γ` −→  0

n→∞

by Markov’s inequality (11.1.4), so almost no  G ∈ G( n, p) lies in  PH.

11.4 Threshold functions and second moments

245

We now come to the second part of the proof: we show that almost

all  G ∈ G( n, p) lie in  PH  if  γ → ∞  as  n → ∞. Note first that, for  n >  k, µ ¶

µ

¶

n

1

n

n−k =

· · · n − k + 1

k

k!

n

n

µ

¶ k

> 1

n − k + 1

k!

n

µ

¶ k

> 1 1  − k −  1

;

(6)

k!

k

¡ ¢

thus,  nk  exceeds  n  by no more than a factor independent of  n.

k

Our goal is to apply Lemma 11.4.2, and hence to bound  σ 2 /µ 2 =

¡

¢

E( X 2)  − µ 2  /µ 2 from above. As in (3) we have X

E( X 2) =

P [  H0 ∪ H00 ⊆ G ]  .

(7)

( H0,H00) ∈H 2

Let us then calculate these probabilities  P [  H0 ∪ H00 ⊆ G ]. Given H0, H00 ∈ H, we have

P [  H0 ∪ H00 ⊆ G ] =  p 2 `−kH0∩H00k.

Since  H  is balanced,  ε( H0 ∩ H00) 6  ε( H) =  `/k. With  |H0 ∩ H00| =:  i i

this yields  kH0 ∩ H00k  6  ì/k, so by 0 6  p  6 1, P [  H0 ∪ H00 ⊆ G ] 6  p 2 `−ì/k.

(8)

We have now estimated the individual summands in (7); what does

this imply for the sum as a whole? Since (8) depends on the parameter

i =  |H0 ∩ H00|, we partition the range  H 2 of the sum in (7) into the subsets

©

ª

H 2 i := ( H0, H00)  ∈ H 2 :  |H0 ∩ H00| =  i , i = 0 , . . . , k,

H 2 i

and calculate for each  H 2 the corresponding sum

i

X

Ai :=

P [  H0 ∪ H00 ⊆ G ]

A

i

i

P

P

by itself. (Here, as below, we use

to denote sums over all pairs

i

i

( H0, H00)  ∈ H 2.)

i

If  i = 0 then  H0  and  H00  are disjoint, so the events  H0 ⊆ G  and H00 ⊆ G  are independent. Hence,

X

A 0 =

P [  H0 ∪ H00 ⊆ G ]

0

246

11. Random Graphs

X

=

P [  H0 ⊆ G ]  · P [  H00 ⊆ G ]

0

X

6

P [  H0 ⊆ G ]  · P [  H00 ⊆ G ]

( H0,H00) ∈H 2

³ X

´ ³ X

´

=

P [  H0 ⊆ G ]  ·

P [  H00 ⊆ G ]

H0 ∈H

H00 ∈H

=  µ 2 .

(9)

(3)

P

P

P

Let us now estimate  A

0

00

i  for  i > 1. Writing

for

H0 ∈H  and

P

P

P

P

0

for

can be written as

0  P 00

. For

H00 ∈H ,  we note that

i

|H0∩H00|= i

P

fixed  H0 (corresponding to the first sum

0), the second sum ranges

over

µ ¶µ

¶

k

n − k h

i

k − i

summands: the number of graphs  H00 ∈ H  with  |H00 ∩ H0| =  i. Hence, for all  i > 1 and suitable constants  c 1 , c 2 independent of  n, X

Ai =

P [  H0 ∪ H00 ⊆ G ]

i

X µ ¶µ

¶

0

6

k

n − k h p 2 `p−ì/k

(8)

i

k − i

µ ¶µ

¶

k

n − k

¡

¢ −ì/k

=  |H|

h p 2 ` γ n−k/`

(2)

i

k − i

6  |H| p`c 1  nk−i h p`γ−ì/k ni

=  µ c 1 nkh p`γ−ì/k

(5)

µ ¶

6

n

µ c 2

h p`γ−ì/k

(6)

k

=  µ 2 c 2 γ−ì/k

(1 ,  5)

6  µ 2 c 2 γ−`/k

if  γ > 1. By definition of the  Ai, this implies with  c 3 :=  kc 2 that ³

k

X

´

E( X 2) /µ 2 =

A 0 /µ 2 +

Ai/µ 2

6 1 +  c 3 γ−`/k

(7)

(9)

i=1

and hence

σ 2

E( X 2)  − µ 2

=

6  c

0  .

µ 2

µ 2

3 γ−`/k −→

γ→∞

11.4 Threshold functions and second moments

247

By Lemma 11.4.2, therefore,  X >  0 almost surely, i.e. almost all  G ∈

G( n, p) have a subgraph isomorphic to  H  and hence lie in  PH.

¤

Theorem 11.4.3 allows us to read off threshold functions for a number of natural graph properties.

Corollary 11.4.4.  If k > 3 , then t( n) =  n− 1  is a threshold function for the property of containing a k-cycle.

¤

Interestingly, the threshold function in Corollary 11.4.4 is indepen-

dent of the cycle length  k  considered: in the evolution of random graphs, cycles of all (constant) lengths appear at about the same time!

There is a similar phenomenon for trees. Here, the threshold func-

tion does depend on the order of the tree considered, but not on its

shape:

Corollary 11.4.5.  If T is a tree of order k > 2 , then t( n) =  n−k/( k− 1) is a threshold function for the property of containing a copy of T .

We finally have the following result for complete subgraphs:

Corollary 11.4.6.  If k > 2 , then t( n) =  n− 2 /( k− 1)  is a threshold function for the property of containing a Kk.

Proof .  Kk  is balanced, because  ε( Ki) = 1 ( i −  1)  <  1 ( k −  1) =  ε( Kk) for 2

2

i < k. With  ` :=  kKkk = 1  k( k −  1), we obtain  n−k/` =  n− 2 /( k− 1).

¤

2

It is not difficult to adapt the proof of Theorem 11.4.3 to the case that  H  is unbalanced. The threshold then becomes  t( n) =  n− 1 /ε0( H), where  ε0( H) := max  { ε( F )  | F ⊆ H }; see Exercise 22.

Exercises

1.  −  What is the probability that a random graph in  G( n, p) has exactly  m

¡ ¢

edges, for 0 6  m  6  n  fixed?

2

2.

What is the expected number of edges in  G ∈ G( n, p)?

3.

What is the expected number of  Kr-subgraphs in  G ∈ G( n, p)?

4.

Characterize the graphs that occur as a subgraph in every graph of

sufficiently large average degree.

5.

In the usual terminology of measure spaces (and in particular, of prob-

ability spaces), the phrase ‘almost all’ is used to refer to a set of points whose complement has measure zero. Rather than considering a limit

of probabilities in  G( n, p) as  n → ∞, would it not be more natural to define a probability space on the set of  all  finite graphs (one copy of each) and to investigate properties of ‘almost all’ graphs in this space, in the sense above?

248

11. Random Graphs

6.

Show that if almost all  G ∈ G( n, p) have a graph property  P 1 and almost all  G ∈ G( n, p) have a graph property  P 2, then almost all  G ∈ G( n, p) have both properties, i.e. have the property  P 1  ∩ P 2.

7.  −  Show that, for constant  p ∈ (0 ,  1), almost every graph in  G( n, p) has diameter 2.

8.

Show that, for constant  p ∈ (0 ,  1), almost no graph in  G( n, p) has a separating complete subgraph.

9.

Derive Proposition 11.3.1 from Lemma 11.3.2.

10.+ (i) Show that with probability 1 an infinite random graph  G ∈ G( ℵ 0 , p) has all the properties  Pi,j ( i, j ∈  N).

(ii) Show that any two (infinite) graphs having all the properties  Pi,j are isomorphic.

(Thus, up to isomorphism, there is only one countably infinite random

graph.)

11.

Let  ² >  0 and  p =  p( n)  >  0, and let  r > (1 +  ²)(2 ln  n) /p  be an integer-valued function of  n. Show that almost no graph in  G( n, p) contains  r independent vertices.

12.

Show that for every graph  H  there exists a function  p =  p( n) such that lim n→∞ p( n) = 0 but almost every  G ∈ G( n, p) contains an induced copy of  H.

13.+ (i) Show that, for every 0  < ²  6 1 and  p = (1  − ²)(ln  n) n− 1, almost every  G ∈ G( n, p) has an isolated vertex.

(ii) Find a probability  p =  p( n) such that almost every  G ∈ G( n, p) is disconnected but the expected number of spanning trees of  G  tends to infinity as  n → ∞.

(Hint for (ii): A theorem of Cayley states that  Kn  has exactly  nn− 2

spanning trees.)

14.+ Given  r ∈  N, find a  c >  0 such that, for  p =  cn− 1, almost every G ∈ G( n, p) has a  Kr  minor. Can  c  be chosen independently of  r?

15.

Find an increasing graph property without a threshold function, and a

property that is not increasing but has a threshold function.

16.  −  Let  H  be a graph of order  k, and let  h  denote the number of graphs isomorphic to  H  on some fixed set of  k  elements. Show that  h  6  k!.

For which graphs  H  does equality hold?

17.  −  For every  k > 1, find a threshold function for  { G | ∆( G) >  k }.

18.  −  Given  d ∈  N, is there a threshold function for the property of containing a  d-dimensional cube (see Ex. 2, Ch. 1)? If so, which; if not, why not?

19.

Show that  t( n) =  n− 1 is also a threshold function for the property of containing  any  cycle.

20.

Does the property of containing any tree of order  k (for  k > 2 fixed) have a threshold function? If so, which?

Exercises

249

21.+ Given a graph  H, let  P  be the property of containing an induced copy of  H. If  H  is complete then, by Corollary 11.4.6,  P  has a threshold function. Show that  P  has no threshold function if  H  is not complete.

22.+ Prove the following version of Theorem 11.4.3 for unbalanced subgraphs. Let  H  be any graph with at least one edge, and put  ε0( H) :=

max  { ε( F )  | ∅ 6=  F ⊆ H }. Then the threshold function for  PH  is t( n) =  n− 1 /ε0( H).

(Hint. Imitate the proof of Theorem 11.4.3. Instead of the sets  Hi, consider the sets  H 2 F :=  { ( H0, H00)  ∈ H 2  | H0 ∩ H00 =  F }. Replace the distinction between the cases of  i = 0 and  i >  0 by the distinction between the cases of  kF k = 0 and  kF k >  0.)

Notes

There are a number of monographs and texts on the subject of random

graphs. The most comprehensive of these is B. Bollobás,  Random Graphs, Academic Press 1985. Another advanced but very readable monograph is

S. Janson, T. ÃLuczak & A. Ruciński,  Topics in Random Graphs, in preparation; this concentrates on areas developed since  Random Graphs  was published. E.M. Palmer,  Graphical Evolution, Wiley 1985, covers material similar to parts of  Random Graphs  but is written in a more elementary way. Compact introductions going beyond what is covered in this chapter are given by B. Bollobás,  Graph Theory, Springer GTM 63, 1979, and by M. Karoński, Handbook of Combinatorics (R.L. Graham, M. Grötschel & L. Lovász, eds.), North-Holland 1995.

A stimulating advanced introduction to the use of random techniques in

discrete mathematics more generally is given by N. Alon & J.H. Spencer,  The Probabilistic Method, Wiley 1992. One of the attractions of this book lies in the way it shows probabilistic methods to be relevant in proofs of entirely deterministic theorems, where nobody would suspect it. Another example for this phenomenon is Alon’s proof of Theorem 5.4.1; see the notes for Chapter 5.

The probabilistic method had its first origins in the 1940s, one of its earliest results being Erd˝

os’s probabilistic lower bound for Ramsey numbers

(Theorem 11.1.3). Lemma 11.3.2 about the properties  Pi,j  is taken from Bollobás’s Springer text cited above. A very readable rendering of the proof that, for constant  p, every first order sentence about graphs is either almost surely true or almost surely false, is given by P. Winkler, Random structures and zero-one laws, in (N.W. Sauer et al., eds.)  Finite and Infinite Combinatorics in Sets and Logic (NATO ASI Series C 411), Kluwer 1993.

The seminal paper on graph evolution is P. Erd˝

os & A. Rényi, On the

evolution of random graphs,  Publ. Math. Inst. Hungar. Acad. Sci. 5 (1960), 17–61. This paper also includes Theorem 11.4.3 and its proof. The generalization of this theorem to unbalanced subgraphs was first proved by Bollobás in 1981, using advanced methods; a simple adaptation of the original Erd˝

os-Renyi

proof was found by Ruciński & Vince (1986), and is presented in Karoński’s Handbook chapter.

250

11. Random Graphs

There is another way of defining a random graph  G, which is just as natural and common as the model we considered. Rather than choosing the edges of  G  independently, we choose the entire graph  G  uniformly at random from among all the graphs on  {  0 , . . . , n −  1  }  that have exactly  M =  M ( n)

¡ ¢

edges: then each of these graphs occurs with the same probability of

N

,

M

¡ ¢

where  N :=

n . Just as we studied the likely properties of the graphs in

2

G( n, p) for different functions  p =  p( n), we may investigate how the properties of  G  in the other model depend on the function  M ( n). If  M  is close to  pN , the expected number of edges of a graph in  G( n, p), then the two models behave very similarly. It is then largely a matter of convenience which of them to consider; see Bollobás for details.

In order to study threshold phenomena in more detail, one often considers the following  random graph process: starting with a  Kn  as stage zero, one chooses additional edges one by one (uniformly at random) until the graph is complete. This is a simple example of a Markov chain, whose  M  th stage corresponds to the ‘uniform’ random graph model described above. A survey about threshold phenomena in this setting is given by T. ÃLuczak, The phase transition in a random graph, in (D. Miklós, V.T. Sós & T. Sz˝

onyi, eds.)  Paul

Erd˝

os is 80, Vol. 2, Proc. Colloq. Math. Soc. János Bolyai (1996).

12

Minors,

Trees,

and WQO

Our goal in this last chapter is a single theorem, one which dwarfs any other result in graph theory and may doubtless be counted among the

deepest theorems that mathematics has to offer:  in every infinite set of graphs there are two such that one is a minor of the other.  This  graph minor theorem (or  minor theorem  for short), inconspicuous though it may look at first glance, has made a fundamental impact both outside

graph theory and within. Its proof, due to Neil Robertson and Paul

Seymour, takes well over 500 pages.

So we have to be modest: of the actual proof of the minor theorem,

this chapter will convey only a very rough impression. However, as with most truly fundamental results, the proof has sparked off the development of methods of quite independent interest and potential. This is

true particularly for the use of  tree-decompositions, a technique we shall meet in Sections 12.3 and 12.4. Section 12.1 gives an introduction to  well-quasi-ordering, a concept central to the minor theorem. In Section 12.2

we apply this concept to prove the minor theorem for trees. The chapter finishes with an overview in Section 12.5 of the proof of the general graph minor theorem, and of some of its immediate consequences.

12.1 Well-quasi-ordering

A reflexive and transitive relation is called a  quasi-ordering. A quasi-ordering 6 on  X  is a  well-quasi-ordering, and the elements of  X  are well-quasi-ordering

well-quasi-ordered  by 6, if for every infinite sequence  x 0 , x 1 , . . .  in  X

252

12. Minors, Trees, and WQO

good pair

there are indices  i < j  such that  xi  6  xj. Then ( xi, xj) is a  good pair of this sequence. A sequence containing a good pair is a  good sequence; good/bad

thus, a quasi-ordering on  X  is a well-quasi-ordering if and only if every sequence

infinite sequence in  X  is good. An infinite sequence is  bad  if it is not good.

Proposition 12.1.1.  A quasi-ordering  6  on X is a well-quasi-ordering if and only if X contains neither an infinite antichain nor an infinite strictly decreasing sequence x 0  > x 1  > . . ..

(9.1.2)

Proof . The forward implication is trivial. Conversely, let  x 0 , x 1 , . . .

be any infinite sequence in  X. Let  K  be the complete graph on N =

{  0 ,  1 , . . . }. Colour the edges  ij ( i < j) of  K  with three colours: green if  xi  6  xj, red if  xi > xj, and amber if  xi, xj  are incomparable. By

Ramsey’s theorem (9.1.2),  K  has an infinite induced subgraph whose edges all have the same colour. If there is neither an infinite antichain nor an infinite strictly decreasing sequence in  X, then this colour must be green, i.e.  x 0 , x 1 , . . .  has an infinite subsequence in which every pair is good. In particular, the sequence  x 0 , x 1 , . . .  is good.

¤

In the proof of Proposition 12.1.1, we showed more than was needed:

rather than finding a single good pair in  x 0 , x 1 , . . . , we found an infinite increasing subsequence. We have thus shown the following:

Corollary 12.1.2.  If X is well-quasi-ordered, then every infinite sequence in X has an infinite increasing subsequence.

¤

The following lemma, and the idea of its proof, are fundamental to

the theory of well-quasi-ordering. Let 6 be a quasi-ordering on a set  X.

6

For finite subsets  A, B ⊆ X, write  A  6  B  if there is an injective mapping f :  A → B  such that  a  6  f ( a) for all  a ∈ A. This naturally extends 6 to

[ X] <ω

a quasi-ordering on [ X] <ω, the set of all finite subsets of  X.

[ 12.2.1 ]

Lemma 12.1.3.  If X is well-quasi-ordered by  6 , then so is [ X] <ω.

Proof . Suppose that 6 is a well-quasi-ordering on  X  but not on [ X] <ω.

We start by constructing a bad sequence ( An) n∈ N in [ X] <ω, as follows.

Given  n ∈  N, assume inductively that  Ai  has been defined for every i < n, and that there exists a bad sequence in [ X] <ω  starting with A 0 , . . . , An− 1. (This is clearly true for  n = 0: by assumption, [ X] <ω

contains a bad sequence, and this has the empty sequence as an initial

segment.) Choose  An ∈ [ X] <ω  so that some bad sequence in [ X] <ω  starts with  A 0 , . . . , An  and  |An|  is as small as possible.

Clearly, ( An) n∈ N is a bad sequence in [ X] <ω; in particular,  An 6=  ∅

for all  n. For each  n  pick an element  an ∈ An  and set  Bn :=  An  r  { an }.

12.1 Well-quasi-ordering

253

By Corollary 12.1.2, the sequence ( an) n∈ N has an infinite increasing subsequence ( an )

, the sequence

i

i∈ N. By the minimal choice of  An 0

A 0 , . . . , An

, B , B , . . .

0  − 1 , Bn 0

n 1

n 2

is good; consider a good pair. Since ( An) n∈ N is bad, this pair cannot have the form ( Ai, Aj) or ( Ai, Bj), as  Bj  6  Aj. So it has the form ( Bi, Bj). Extending the injection  Bi → Bj  by  ai 7→ aj, we deduce again that ( Ai, Aj) is good, a contradiction.

¤

12.2 The graph minor theorem for trees

The minor theorem can be expressed by saying that the finite graphs

are well-quasi-ordered by the minor relation 4. Indeed, by Proposi-

tion 12.1.1 and the obvious fact that no strictly descending sequence

of minors can be infinite, being well-quasi-ordered is equivalent to the non-existence of an infinite antichain, the formulation used earlier.

In this section, we prove a strong version of the graph minor theorem

for trees:

Theorem 12.2.1. (Kruskal 1960)

The finite trees are well-quasi-ordered by the topological minor relation.

We shall base the proof of Theorem 12.2.1 on the following notion

of an embedding between rooted trees, which strengthens the usual em-

bedding as a topological minor. Consider two trees  T  and  T 0, with roots r  and  r0  say. Let us write  T  6  T 0  if there exists an isomorphism  ϕ, from T  6  T 0

some subdivision of  T  to a subtree  T 00  of  T 0, that preserves the tree-order on  V ( T ) associated with  T  and  r. (Thus if  x < y  in  T  then  ϕ( x)  < ϕ( y) in  T 0; see Fig. 12.2.1.) As one easily checks, this is a quasi-ordering on the class of all rooted trees.

ϕ( r)

ϕ

T

r

T 0

r0

Fig. 12.2.1.  An embedding of  T  in  T 0  showing that  T  6  T 0

254

12. Minors, Trees, and WQO

(12.1.3)

Proof of Theorem 12.2.1. We show that the rooted trees are well-quasi-ordered by the relation 6 defined above; this clearly implies the theorem.

Suppose not. To derive a contradiction, we proceed as in the proof

of Lemma 12.1.3. Given  n ∈  N, assume inductively that we have chosen a sequence  T 0 , . . . , Tn− 1 of rooted trees such that some bad sequence of Tn

rooted trees starts with this sequence. Choose as  Tn  a minimum-order rooted tree such that some bad sequence starts with  T 0 , . . . , Tn. For each rn

n ∈  N, denote the root of  Tn  by  rn.

An

Clearly, ( Tn) n∈ N is a bad sequence. For each  n, let  An  denote the set of components of  Tn − rn, made into rooted trees by choosing the neighbours of  rn  as their roots. Note that the tree-order of these trees S

A

is that induced by  Tn. Let us prove that the set  A :=

n∈ N  An  of all

these trees is well-quasi-ordered.

T k

Let ( T k) k∈ N be any sequence of trees in  A. For every  k ∈  N choose n( k)

an  n =  n( k) such that  T k ∈ An. Pick a  k  with smallest  n( k). Then T 0 , . . . , Tn( k) − 1 , T k, T k+1 , . . .

is a good sequence, by the minimal choice of  Tn( k) and  T k $  Tn( k). Let ( T, T 0) be a good pair of this sequence. Since ( Tn) n∈ N is bad,  T  cannot be among the first  n( k) members  T 0 , . . . , Tn( k) − 1 of our sequence: then T 0  would be some  T i  with  i >  k, i.e.

T  6  T 0 =  T i  6  Tn( i) ;

since  n( k) 6  n( i) by the choice of  k, this would make ( T, Tn( i)) a good pair in the bad sequence ( Tn) n∈ N. Hence ( T, T 0) is a good pair also in ( T k) k∈ N, completing the proof that  A  is well-quasi-ordered.

By Lemma 12.1.3,1 the sequence ( An) n∈ N in [ A] <ω  has a good pair i, j

( Ai, Aj); let  f :  Ai → Aj  be an injection such that  T  6  f ( T ) for all  T ∈ Ai.

We now extend the union of the embeddings  T → f ( T ) to a map  ϕ  from V ( Ti) to  V ( Tj) by letting  ϕ( ri) :=  rj. This map  ϕ  preserves the tree-order of  Ti, and it defines an embedding to show that  Ti  6  Tj, since the edges  rir ∈ Ti  map naturally to the paths  rjTjϕ( r). Hence ( Ti, Tj) is a good pair in our original bad sequence of rooted trees, a contradiction.

¤

1 Any readers worried that we might need the lemma for sequences or multi-sets rather than just sets here, please note that isomorphic elements of  An  are not identified: we always have  |An| =  d( rn).

12.3 Tree-decompositions

255

12.3 Tree-decompositions

Trees are graphs with some very distinctive and fundamental properties; consider Theorem 1.5.1 and Corollary 1.5.2, or the more sophisticated example of Kruskal’s theorem. It is therefore legitimate to ask to what degree those properties can be transferred to more general graphs, graphs that are not themselves trees but tree-like in some sense.2 In this section, we study a concept of tree-likeness that permits generalizations of all the tree properties referred to above (including Kruskal’s theorem), and which plays a crucial role in the proof of the graph minor theorem.

Let  G  be a graph,  T  a tree, and let  V = ( Vt) t∈T  be a family of vertex sets  Vt ⊆ V ( G) indexed by the vertices  t  of  T . The pair ( T, V) is called a  tree-decomposition  of  G  if it satisfies the following three conditions: treeS

decomposition

(T1)  V ( G) =

V

t∈T

t;

(T2) for every edge  e ∈ G  there exists a  t ∈ T  such that both ends of  e lie in  Vt;

(T3)  Vt ∩ V

⊆ V  whenever  t

1

t 3

t 2

1 , t 2 , t 3  ∈ T  satisfy  t 2  ∈ t 1 T t 3.

Conditions (T1) and (T2) together say that  G  is the union of the subgraphs  G [  Vt ]; we call these subgraphs and the sets  Vt  themselves the parts  of ( T, V) and say that ( T, V) is a tree-decomposition of  G into  these parts

parts. Condition (T3) implies that the parts of ( T, V) are organized into

roughly like a tree (Fig. 12.3.1).

t 1

Vt ?

3

t 3

e?

t?

?

t 2

T

G

Fig. 12.3.1.  Edges and parts ruled out by (T2) and (T3)

Before we discuss the role that tree-decompositions play in the proof

of the minor theorem, let us note some of their basic properties. Consider a fixed tree-decomposition ( T, V) of  G, with  V = ( Vt) t∈T  as above.

( T, V) , Vt

Perhaps the most important feature of a tree-decomposition is that

it transfers the separation properties of its tree to the graph decomposed: 2 What exactly this ‘sense’ should be will depend both on the property considered and on its intended application.

256

12. Minors, Trees, and WQO

Lemma 12.3.1.  Let t 1 t 2  be any edge of T and let T 1 , T 2  be the components of T − t 1 t 2 , with t 1  ∈ T 1  and t 2  ∈ T 2 . Then Vt ∩ V

separates

1

t 2

S

S

U 1 :=

V

V

t∈T

t from U 2 :=

t in G (Fig. 12.3.2).

1

t∈T 2

T 2

T 1

t

U

2

2

U 1

t 1

Vt ∩ V

1

t 2

Fig. 12.3.2. Vt ∩ V

separates  U

1

t 2

1 from  U 2 in  G

Proof . Both  t 1 and  t 2 lie on every  t– t0  path in  T  with  t ∈ T 1 and  t0 ∈ T 2.

Therefore  U 1  ∩ U 2  ⊆ Vt ∩ V

by (T3), so all we have to show is that  G

1

t 2

has no edge  u 1 u 2 with  u 1  ∈ U 1 r  U 2 and  u 2  ∈ U 2 r  U 1. If  u 1 u 2 is such an edge, then by (T2) there is a  t ∈ T  with  u 1 , u 2  ∈ Vt. By the choice of u 1 and  u 2 we have neither  t ∈ T 2 nor  t ∈ T 1, a contradiction.

¤

Note that tree-decompositions are passed on to subgraphs:

¡

¢

[ 12.4.2 ]

Lemma 12.3.2.  For every H ⊆ G, the pair T, ( Vt ∩ V ( H)) t∈T is a tree-decomposition of H.

¤

Similarly for contractions:

Lemma 12.3.3.  Suppose that G is an M H with branch sets Uh,

h ∈ V ( H) . Let f :  V ( G)  → V ( H)  be the map assigning to each vertex of G the index of the branch set containing it. For all t ∈ T let Wt :=  { f ( v)  | v ∈ Vt }, and put W := ( Wt) t∈T . Then ( T, W)  is a tree-decomposition of H.

Proof . The assertions (T1) and (T2) for ( T, W) follow immediately from the corresponding assertions for ( T, V). Now let  t 1 , t 2 , t 3  ∈ T  be as in (T3), and consider a vertex  h ∈ Wt ∩ W

of  H; we show that

1

t 3

h ∈ Wt . By definition of  W

and  W , there are vertices  v

∩ U

2

t 1

t 3

1  ∈ Vt 1

h

and  v 3  ∈ Vt ∩ U

separates  v

3

h. Since  Uh  is connected in  G  and  Vt 2

1 from

v 3 in  G  by Lemma 12.3.1,  Vt  has a vertex in  U

,

2

h. By definition of  Wt 2

this implies  h ∈ Wt .

¤

2

Here is another useful consequence of Lemma 12.3.1:

12.3 Tree-decompositions

257

Lemma 12.3.4.  Given a set W ⊆ V ( G) , there is either a t ∈ T such that W ⊆ Vt, or there are vertices w 1 , w 2  ∈ W and an edge t 1 t 2  ∈ T such that w 1 , w 2  lie outside the set Vt ∩ V

and are separated by it in G.

1

t 2

Proof . Let us orient the edges of  T  as follows. For each edge  t 1 t 2  ∈ T , define  U 1 , U 2 as in Lemma 12.3.1; then  Vt ∩ V

separates  U

1

t 2

1 from  U 2.

If  Vt ∩ V

does not separate any two vertices of  W  that lie outside it,

1

t 2

we can find an  i ∈ {  1 ,  2  }  such that  W ⊆ Ui, and orient  t 1 t 2 towards  ti.

Let  t  be the last vertex of a maximal directed path in  T ; we claim that  W ⊆ Vt. Given  w ∈ W , let  t0 ∈ T  be such that  w ∈ Vt0. If  t0 6=  t, then the edge  e  at  t  that separates  t0  from  t  in  T  is directed towards  t, so  w  also lies in  Vt00  for some  t00  in the component of  T − e  containing  t.

Therefore  w ∈ Vt  by (T3).

¤

The following special case of Lemma 12.3.4 is used particularly often:

Lemma 12.3.5.  Any complete subgraph of G is contained in some part

[ 12.4.2 ]

of ( T, V) .

¤

As indicated by Figure 12.3.1, the parts of ( T, V) reflect the structure of the tree  T , so in this sense the graph  G  decomposed resembles a tree. However, this is valuable only inasmuch as the structure of  G  within each part is negligible: the smaller the parts, the closer the resemblance.

This observation motivates the following definition. The  width  of width

( T, V) is the number

©

ª

max  |Vt| −  1 :  t ∈ T ,

and the  tree-width  tw( G) of  G  is the least width of any tree-decomposi-tree-width

tw( G)

tion of  G. As one easily checks,3 trees themselves have tree-width 1.

By Lemmas 12.3.2 and 12.3.3, the tree-width of a graph will never be increased by deletion or contraction:

Proposition 12.3.6.  If H  4  G then  tw( H) 6 tw( G) .

¤

Graphs of bounded tree-width are sufficiently similar to trees that it

becomes possible to adapt the proof of Kruskal’s theorem to the class of these graphs; very roughly, one has to iterate the ‘minimal bad sequence’

argument from the proof of Lemma 12.1.3 tw( G) times. This takes us a step further towards a proof of the graph minor theorem:

Theorem 12.3.7. (Robertson & Seymour 1990)

For every integer k >  0 , the graphs of tree-width < k are well-quasi-ordered by the minor relation.

3 Indeed the ‘ − 1’ in the definition of width serves no other purpose than to make this statement true.

258

12. Minors, Trees, and WQO

In order to make use of Theorem 12.3.7 for a proof of the general

minor theorem, we should be able to say something about the graphs it does not cover, i.e. to deduce some information about a graph from the

assumption that its tree-width is large. Our next theorem achieves just that: it identifies a canonical obstruction to small tree-width, a structural phenomenon that occurs in a graph if and only if its tree-width is large.

touch

Let us say that two subsets of  V ( G)  touch  if they have a vertex in common or  G  contains an edge between them. A set of mutually touching bramble

connected vertex sets in  G  is a  bramble. Extending our terminology of cover

Chapter 2.1, we say that a subset of  V ( G)  covers (or is a  cover  of) a bramble  B  if it meets every element of  B. The least number of vertices order

covering a bramble is the  order  of that bramble.

The following simple observation will be useful:

Lemma 12.3.8.  Any set of vertices separating two covers of a bramble also covers that bramble.

Proof . Since each set in the bramble is connected and meets both of the covers, it also meets any set separating these covers.

¤

A typical example of a bramble is the set of crosses in a grid. The

grid

k × k grid  is the graph on  {  1 , . . . , k } 2 with the edge set

{ ( i, j)( i0, j0) :  |i − i0| +  |j − j0| = 1  } .

The  crosses  of this grid are the  k 2 sets

Cij :=  { ( i, `)  | ` = 1 , . . . , k } ∪ { ( `, j)  | ` = 1 , . . . , k } .

Thus, the cross  Cij  is the union of the grid’s  i th column and its  j th row.

Clearly, the crosses of the  k × k  grid form a bramble of order  k: they are covered by any row or column, while any set of fewer than  k  vertices misses both a row and a column, and hence a cross.

The following result is sometimes called the  tree-width duality theorem:

Theorem 12.3.9. (Seymour & Thomas 1993)

Let k > 0  be an integer. A graph has tree-width >  k if and only if it contains a bramble of order > k.

(3.3.1)

Proof . For the backward implication, let  B  be any bramble in a graph  G.

We show that every tree-decomposition ( T, ( Vt) t∈T ) of  G  has a part that meets every set in  B.

As in the proof of Lemma 12.3.4 we start by orienting the edges  t 1 t 2

of  T . If  X :=  Vt ∩ V

meets every  B ∈ B, we are done. If not, then

1

t 2

12.3 Tree-decompositions

259

for each  B  disjoint from  X  there is an  i ∈ {  1 ,  2  }  such that  B ⊆ Ui  r  X

(defined as in Lemma 12.3.1); recall that  B  is connected. Moreover, this i  is the same for all such  B, because they touch. We now orient the edge t 1 t 2 towards  ti.

If every edge of  T  is oriented in this way and  t  is the last vertex of a maximal directed path in  T , then  Vt  meets every set in  B—just as in the proof of Lemma 12.3.4.

To prove the forward direction, we now assume that  G  contains no bramble of order  > k. We show that for every bramble  B  in  G  there is B-a  B-admissible  tree-decomposition of  G, one in which any part of order admissible

> k  fails to cover  B. For  B =  ∅  this implies that tw( G)  < k, because every set covers the empty bramble.

Let  B  be given, and assume inductively that for every bramble  B0

B

with more sets than  B  there is a  B0-admissible tree-decomposition of  G.

(The induction starts, since no bramble in  G  has more than 2 |G|  sets.) Let  X ⊆ V ( G) be a cover of  B  with as few vertices as possible; then X

` :=  |X|  6  k  is the order of  B. Our aim is to show the following:

`

For every component C of G − X there exists a B-admissible

( ∗)

tree-decomposition of G [  X ∪ V ( C) ]  with X as a part.

Then these tree-decompositions can be combined to a  B-admissible tree-decomposition of  G  by identifying their nodes corresponding to  X. (If X =  V ( G), then the tree-decomposition with  X  as its only part is  B-

admissible.)

So let  C  be a fixed component of  G − X, write  H :=  G [  X ∪ V ( C) ], C, H

and put  B0 :=  B ∪ { C }. If  B0  is not a bramble then  C  fails to touch B0

some element of  B, and hence  Y :=  V ( C)  ∪ N ( C) does not cover  B.

Then the tree-decomposition of  H  consisting of the two parts  X  and  Y

satisfies ( ∗).

So we may assume that  B0  is a bramble. Since  X  covers  B  but not  B0, we have  |B0| > |B|. Our induction hypothesis therefore ensures that  G  has a  B0-admissible tree-decomposition ( T, ( Vt) t∈T ). If this de-T, ( Vt) t∈T

composition is also  B-admissible, there is nothing more to show. If not, then one of its parts of order  > k,  Vs  say, covers  B. Since no set of fewer s

than  `  vertices covers  B, Lemma 12.3.8 implies with Menger’s theorem

(3.3.1) that  Vs  and  X  are linked by  `  disjoint paths  P 1 , . . . , P`. As  Vs Pi

fails to cover  B0  and hence lies in  G − C, the paths  Pi  meet  H  only in their ends  xi ∈ X.

xi

For each  i = 1 , . . . , `  pick a  ti ∈ T  with  xi ∈ Vt , and let t

i

i

Wt := ( Vt ∩ ( X ∪ V ( C)))  ∪ { xi | t ∈ sT ti } ; for all  t ∈ T (Fig. 12.3.3). Then ( T, ( Wt) t∈T ) is the tree-decomposition which ( T, ( Vt) t∈T ) induces on  H (cf. Lemma 12.3.2), except that a few

260

12. Minors, Trees, and WQO

Vt0

Vt 2

x 1

x 2

V

x

t

3

1

Vt 3

Vt

Vs

Fig. 12.3.3. Wt  contains  x 2 and  x 3 but not  x 1;  Wt0  contains no  xi xi  have been added to some of the parts. Despite these additions, we still have  |Wt|  6  |Vt|  for all  t: for each  xi ∈ Wt  r  Vt  we have  t ∈ sT ti, so  Vt  contains some other vertex of  Pi (Lemma 12.3.1); that vertex does not lie in  Wt, because  Pi  meets  H  only in  xi. Moreover, ( T, ( Wt) t∈T ) clearly satisfies (T3), because each  xi  is added to every part along some path in  T , so it is again a tree-decomposition.

As  Ws =  X, all that is left to show for ( ∗) is that this decomposition is  B-admissible. Consider any  Wt  of order  > k. Then  Wt  meets  C, because  |X| =  `  6  k. Since ( T, ( Vt) t∈T ) is  B0-admissible and  |Vt| >

|Wt| > k, we know that  Vt  fails to meet some  B ∈ B; let us show that  Wt does not meet this  B  either. If it does, it must do so in some  xi ∈ Wt  r  Vt.

Then  B  is a connected set meeting both  Vs  and  Vt  but not  V

i

t. As  t ∈ sT ti

by definition of  Wt, this contradicts Lemma 12.3.1.

¤

Often, Theorem 12.3.9 is stated in terms of the  bramble number  of a graph, the largest order of any bramble in it. The theorem then says that the tree-width of a graph is exactly one less than its bramble number

(Exercise 15).

How useful even the easy backward direction of Theorem 12.3.9 can be is exemplified once more by our example of the crosses bramble in the k × k  grid: this bramble has order  k, so by the theorem the  k × k  grid has tree-width at least  k −  1. (Try to show this without the theorem!) In fact, the  k × k  grid has tree-width  k (Exercise 16). But more important than its precise value is the fact that the tree-width of grids tends to infinity with their size. For as we shall see, large grid minors pose another canonical obstruction to small tree-width: not only do

large grids (and hence all graphs containing large grids as minors; cf.

Proposition 12.3.6) have large tree-width, but conversely every graph of large tree-width has a large grid minor (Theorem 12.4.4).

Yet another canonical obstruction to small tree-width is described

in Exercise 30.

12.3 Tree-decompositions

261

Given any two vertices  t 1 , t 2  ∈ T , Lemma 12.3.1 implies that every Vt  with  t ∈ t 1 T t 2 separates  Vt  from  V

in  G. Let us call our tree-

1

t 2

decomposition ( T, V) of  G linked, or  lean,4 if it satisfies the following  linked/lean condition:

(T4) given any  s ∈  N and  t 1 , t 2  ∈ T , either  G  contains  s  disjoint  Vt – V

1

t 2

paths or there exists a  t ∈ t 1 T t 2 such that  |Vt| < s.

The ‘branches’ in a lean tree-decomposition are thus stripped of any bulk not necessary to maintain their connecting qualities: if a branch is thick (the parts along a path in  T  large), then  G  is highly connected along this branch.

In our quest for tree-decompositions into ‘small’ parts, we now have

two criteria to choose between: the global ‘worst case’ criterion of width, which ensures that  T  is nontrivial (unless  G  is complete) but says nothing about the tree-likeness of  G  among parts other than the largest, and the more subtle local criterion of leanness, which ensures tree-likeness everywhere along  T  but might be difficult to achieve except with trivial or near-trivial  T . Surprisingly, though, it is always possible to find a tree-decomposition that is optimal with respect to both criteria at once: Theorem 12.3.10. (Thomas 1990)

Every graph G has a lean tree-decomposition of width  tw( G) .

The proof of Theorem 12.3.10 is not too long but technical, and we shall not present it. The fact that this theorem gives us a very useful property of minimum-width tree-decompositions ‘for free’ has made it a valuable

tool wherever tree-decompositions are applied.

The tree-decomposition ( T, V) of  G  is called  simplicial  if all the simplicial

separators  Vt ∩ V

induce complete subgraphs in  G. This assumption

1

t 2

can enable us to lift assertions about the parts of the decomposition to G  itself. For example, if all the parts in a simplicial tree-decomposition of  G  are  k-colourable, then so is  G (proof?). The same applies to the property of not containing a  Kr  minor for some fixed  r. Algorithmically, it is easy to obtain a simplicial tree-decomposition of a given graph into irreducible parts. Indeed, all we have to do is split the graph recursively along complete separators; if these are always chosen minimal, then the set of parts obtained will even be unique (Exercise 22).

Conversely, if  G  can be constructed recursively from a set  H  of graphs by pasting along complete subgraphs, then  G  has a simplicial tree-decomposition into elements of  H. For example, by Wagner’s Theorem 8.3.4, any graph without a  K 5 minor has a supergraph with a simplicial tree-decomposition into plane triangulations and copies of the 4 depending on which of the two dual aspects of (T4) we wish to emphasize

262

12. Minors, Trees, and WQO

Wagner graph  W , and similarly for graphs without  K 4 minors (see Proposition 12.4.2).

Tree-decompositions may thus lead to intuitive structural charac-

terizations of graph properties. A particularly simple example is the

following characterization of chordal graphs:

[ 12.4.2 ]

Proposition 12.3.11.  G is chordal if and only if G has a tree-decomposition into complete parts.

(5.5.1)

Proof . We apply induction on  |G|. We first assume that  G  has a tree-decomposition ( T, V) such that  G [  Vt ] is complete for every  t ∈ T ; let us choose ( T, V) with  |T |  minimal. If  |T |  6 1, then  G  is complete and hence chordal. So let  t 1 t 2  ∈ T  be an edge, and for  i = 1 ,  2 define  Ti and  Gi :=  G [  Ui ] as in Lemma 12.3.1. Then  G =  G 1  ∪ G 2 by (T1) and (T2), and  V ( G 1  ∩ G 2) =  Vt ∩ V

by the lemma; thus,  G

1

t 2

1  ∩ G 2 is

complete. Since ( Ti, ( Vt) t∈T ) is a tree-decomposition of  G

i

i  into complete

parts, both  Gi  are chordal by the induction hypothesis. (By the choice of ( T, V), neither  Gi  is a subgraph of  G [  Vt ∩ V ] =  G

1

t 2

1  ∩ G 2, so both

Gi  are indeed smaller than  G.) Since  G 1  ∩ G 2 is complete, any induced cycle in  G  lies in  G 1 or in  G 2 and hence has a chord, so  G  too is chordal.

Conversely, assume that  G  is chordal. If  G  is complete, there is nothing to show. If not then, by Proposition 5.5.1,  G  is the union of smaller chordal graphs  G 1 , G 2 with  G 1  ∩ G 2 complete. By the induction hypothesis,  G 1 and  G 2 have tree-decompositions ( T 1 , V 1) and ( T 2 , V 2) into complete parts. By Lemma 12.3.5,  G 1  ∩ G 2 lies inside one of those parts in each case, say with indices  t 1  ∈ T 1 and  t 2  ∈ T 2. As one easily checks, (( T 1  ∪ T 2) +  t 1 t 2 , V 1  ∪ V 2) is a tree-decomposition of  G  into complete parts.

¤

©

ª

Corollary 12.3.12. tw( G) = min  ω( H)  −  1  | G ⊆ H;  H  chordal  .

Proof . By Lemma 12.3.5 and Proposition 12.3.11, each of the graphs  H

considered for the minimum has a tree-decomposition of width  ω( H)  −  1.

Every such tree-decomposition induced one of  G  by Lemma 12.3.2, so tw( G) 6  ω( H)  −  1 for every  H.

Conversely, let us construct an  H  as above with  ω( H)  −  1 6 tw( G).

Let ( T, V) be a tree-decomposition of  G  of width tw( G). For every  t ∈ T

S

let  Kt  denote the complete graph on  Vt, and put  H :=

K

t∈T

t. Clearly,

( T, V) is also a tree-decomposition of  H. By Proposition 12.3.11,  H  is chordal, and by Lemma 12.3.5,  ω( H)  −  1 is at most the width of ( T, V), i.e. at most tw( G).

¤

12.4 Tree-width and forbidden minors

263

12.4 Tree-width and forbidden minors

If  H  is any set or class of graphs, then the class

Forb4( H) :=  { G | G 6<  H  for all  H ∈ H }

Forb4( H)

of all graphs without a minor in  H  is a graph property, i.e. is closed under isomorphism.5 When it is written as above, we say that this property

forbidden

is expressed by specifying the graphs  H ∈ H  as  forbidden (or  excluded) minors

minors.

By Proposition 1.7.3, Forb4( H) is closed under taking minors: if

(1.7.3)

G0  4  G ∈  Forb4( H) then  G0 ∈  Forb4( H). Graph properties that are closed under taking minors will be called  hereditary  in this chapter.

hereditary

Every hereditary property can in turn be expressed by forbidden minors: Proposition 12.4.1.  A graph property P can be expressed by forbidden

[ 12.5.1 ]

minors if and only if it is hereditary.

Proof . For the ‘if’ part, note that  P = Forb4( P), where  P  is the P

complement of  P.

¤

In Section 12.5, we shall return to the general question of how a given hereditary property is best represented by forbidden minors. In this

section, we are interested in one particular type of hereditary property: bounded tree-width.

Thus, let us consider the property of having tree-width less than

some given integer  k. By Propositions 12.3.6 and 12.4.1, this property can be expressed by forbidden minors. Choosing their set  H  as small as possible, we find that  H =  { K 3  }  for  k = 2: the graphs of tree-width

<  2 are precisely the forests. For  k = 3, we have  H =  { K 4  }: Proposition 12.4.2.  A graph has tree-width <  3  if and only if it has no K 4  minor.

(8.3.1)

Proof . By Lemma 12.3.5, we have tw( K 4) > 3. By Proposition 12.3.6,

(12.3.2)

(12.3.5)

therefore, a graph of tree-width  <  3 cannot contain  K 4 as a minor.

(12.3.11)

Conversely, let  G  be a graph without a  K 4 minor; we assume that

|G| > 3. Add edges to  G  until the graph  G0  obtained is edge-maximal without a  K 4 minor. By Proposition 8.3.1,  G0  can be constructed recursively from triangles by pasting along  K 2s. By induction on the number of recursion steps and Lemma 12.3.5, every graph constructible in this way has a tree-decomposition into triangles (as in the proof of

Proposition 12.3.11). Such a tree-decomposition of  G0  has width 2, and by Lemma 12.3.2 it is also a tree-decomposition of  G.

¤

5 As usual, we abbreviate Forb4( { H }) to Forb4( H).

264

12. Minors, Trees, and WQO

A question converse to the above is to ask for which  H (other than K 3 and  K 4) the tree-width of the graphs in Forb4( H) is bounded. Interestingly, it is not difficult to show that any such  H  must be planar.

(4.4.6)

Indeed, as all grids and their minors are planar (why?), every class

Forb4( H) with non-planar  H  contains all grids; yet as we saw after

Theorem 12.3.9, the grids have unbounded tree-width.

The following deep and surprising theorem says that, conversely, the

tree-width of the graphs in Forb4( H) is bounded for every planar  H: Theorem 12.4.3. (Robertson & Seymour 1986)

Given a graph H, the graphs without an H minor have bounded tree-

width if and only if H is planar.

The rest of this section is devoted to the proof of Theorem 12.4.3.

To prove Theorem 12.4.3 we have to show that forbidding any planar

graph  H  as a minor bounds the tree-width of a graph. In fact, we only have to show this for the special cases when  H  is a grid, because every planar graph is a minor of some grid. (To see this, take a drawing of the graph, fatten its vertices and edges, and superimpose a sufficiently fine plane grid.) It thus suffices to show the following:

Theorem 12.4.4. (Robertson & Seymour 1986)

For every integer r there is an integer k such that every graph of tree-width at least k has an r × r grid minor.

Our proof of Theorem 12.4.4, which is much shorter than the original

proof, proceeds as follows. Let  r  be given, and let  G  be any graph of large enough tree-width (depending on  r). We first show that  G  contains a large family  A =  { A 1 , . . . , Am }  of disjoint connected vertex sets such that each pair  Ai, Aj ∈ A  can be linked in  G  by a family  Pij  of many disjoint  Ai– Aj  paths avoiding all the other sets in  A. We then consider all the pairs ( Pij, Pi0j0) of these path families. If we can find a pair among these such that many of the paths in  Pij  meet many of the paths in  Pi0j0, we shall think of the paths in  Pij  as horizontal and the paths in  Pi0j0  as vertical and extract a subdivision of an  r × r  grid from their union. (This will be the difficult part of the proof, because these paths will in general meet in a less orderly way than they do in a grid.) If

not, then for every pair ( Pij, Pi0j0) many of the paths in  Pij  avoid many of the paths in  Pi0j0. We can then select one path  Pij ∈ Pij  from each family so that these selected paths are pairwise disjoint. Contracting

each of the connected sets  A ∈ A  will then give us a  Km  minor in  G, which contains the desired  r × r  grid if  m >  r 2.

To implement these ideas formally, we need a few definitions. Let

externally

us call a set  X ⊆ V ( G)  externally k-connected  in  G  if  |X| ≥ k  and for k-connected  all disjoint subsets  Y, Z ⊆ X  with  |Y | =  |Z| ≤ k  there are  |Y |  disjoint

12.4 Tree-width and forbidden minors

265

Y – Z  paths in  G  that have no inner vertex or edge in  G [  X ]. Note that the vertex set of a  k-connected subgraph of  G  need not be externally k-connected in  G. On the other hand, any horizontal path in the  r × r grid is externally  k-connected in that grid for every  k  6  r. (How?) One of the first things we shall prove below is that any graph of

large enough tree-width—not just grids—contains a large externally  k-

connected set of vertices (Lemma 12.4.5). Conversely, it is easy to show that large externally  k-connected sets (with  k  large) can exist only in graphs of large tree-width (Exercise 30). So, like large grid minors, these sets form a canonical obstruction to small tree-width: they can be found in a graph if and only if its tree-width is large.

An ordered pair ( A, B) of subgraphs of  G  will be called a  premesh premesh

in  G  if  G =  A ∪ B  and  A  contains a tree  T  such that (i)  T  has maximum degree  ≤  3;

(ii) every vertex of  A ∩ B  lies in  T  and has degree  ≤  2 in  T ; (iii)  T  has a leaf in  A ∩ B, or  |T | = 1 and  T ⊆ A ∩ B.

The  order  of such a premesh is the number  |A ∩ B|, and if  V ( A ∩ B) is order

externally  k-connected in  B  then this premesh is a  k-mesh  in  G.

k-mesh

Lemma 12.4.5.  Let G be a graph and let h ≥ k ≥  1  be integers. If G

contains no k-mesh of order h then G has tree-width < h +  k −  1 .

Proof . We may assume that  G  is connected. Let  U ⊆ V ( G) be max-U

imal such that  G [  U ] has a tree-decomposition  D  of width  < h +  k −  1

D

with the additional property that, for every component  C  of  G − U , the neighbours of  C  in  U  lie in one part of  D  and ( G − C, ˜

C) is a premesh

of order  ≤ h, where ˜

C :=  G [  V ( C)  ∪ N ( C) ]. Clearly,  U 6=  ∅.

˜

C

We claim that  U =  V ( G). Suppose not. Let  C  be a component of C

G − U , put  X :=  N ( C), and let  T  be a tree associated with the premesh X

( G − C, ˜

C).

T

By assumption,  |X| ≤ h; let us show that equality holds here. If not, let  u ∈ X  be a leaf of  T (respectively  { u } :=  V ( T )) as in (iii), and let  v ∈ C  be a neighbour of  u. Put  U 0 :=  U ∪ { v }  and  X0 :=  X ∪ { v }, let  T 0  be the tree obtained from  T  by joining  v  to  u, and let  D0  be the tree-decomposition of  G [  U 0 ] obtained from  D  by adding  X0  as a new part (joined to a part of  D  containing  X, which exists by our choice of  U ; see Fig. 12.4.1). Clearly  D0  still has width  < h +  k −  1. Consider a component  C0  of  G − U 0. If  C0 ∩ C =  ∅  then  C0  is also a component of G − U , so  N ( C0) lies inside a part of  D (and hence of  D0), and ( G − C0, ˜

C0)

is a premesh of order  ≤ h  by assumption. If  C0 ∩ C 6=  ∅, then  C0 ⊆ C

and  N ( C0)  ⊆ X0. Moreover,  v ∈ N ( C0): otherwise  N ( C0)  ⊆ X  would separate  C0  from  v, contradicting the fact that  C0  and  v  lie in the same component  C  of  G − X. Since  v  is a leaf of  T 0, it is straightforward to

266

12. Minors, Trees, and WQO

C

U

v

C0

u

T

X

Fig. 12.4.1.  Extending  U  and  D  when  |X| < h check that ( G − C0, ˜

C0) is again a premesh of order  ≤ h, contrary to the

maximality of  U .

Thus  |X| =  h, so by assumption our premesh ( G − C, ˜

C) cannot

Y, Z

be a  k-mesh; let  Y, Z ⊆ X  be sets to witness this. Let  P  be a set of as many disjoint  Y – Z  paths in  H :=  G [  V ( C)  ∪ Y ∪ Z ]  − E( G [  Y ∪ Z ]) as possible. Since all these paths are ‘external’ to  X  in ˜

C, we have

k0

k0 :=  |P| < |Y | =  |Z|  6  k  by the choice of  Y  and  Z. By Menger’s

S

theorem (3.3.1),  Y  and  Z  are separated in  H  by a set  S  of  k0  vertices.

Clearly,  S  has exactly one vertex on each path in  P; we denote the path Ps

containing the vertex  s ∈ S  by  Ps (Fig. 12.4.2).

H

Ps

C

Y

T 0

v

s

S

T

Z

U

C0

X

Fig. 12.4.2. S  separates  Y  from  Z  in  H

Let  X0 :=  X ∪ S  and  U 0 :=  U ∪ S, and let  D0  be the tree-decomposition of  G [  U 0 ] obtained from  D  by adding  X0  as a new part.

Clearly,  |X0| ≤ |X| +  |S| ≤ h +  k −  1. We show that  U0  contradicts the maximality of  U .

Since  Y ∪ Z ⊆ N ( C) and  |S| < |Y | =  |Z|  we have  S ∩ C 6=  ∅, so U 0  is larger than  U . Let  C0  be a component of  G − U 0. If  C0 ∩ C =  ∅, we argue as earlier. So  C0 ⊆ C  and  N ( C0)  ⊆ X0. As before,  C0  has at v

least one neighbour  v  in  S ∩ C, since  X  cannot separate  C0 ⊆ C  from S ∩ C. By definition of  S,  C0  cannot have neighbours in both  Y \ S

12.4 Tree-width and forbidden minors

267

and  Z \ S; we assume it has none in  Y \ S. Let  T 0  be the union of  T

and all the  Y – S  subpaths of paths  Ps  with  s ∈ N ( C0)  ∩ C; since these subpaths start in  Y \ S  and have no inner vertices in  X0, they cannot meet  C0. Therefore ( G − C0, ˜

C0) is a premesh with tree  T 0  and leaf  v;

the degree conditions on  T 0  are easily checked. Its order is  |N ( C0) | ≤

|X| − |Y | +  |S| =  h − |Y | +  k0 < h, a contradiction to the maximality of  U .

¤

Lemma 12.4.6.  Let k ≥  2  be an integer. Let T be a tree of maximum degree  6 3  and X ⊆ V ( T ) . Then T has a set F of edges such that every component of T − F has between k and  2 k −  1  vertices in X, except that one such component may have fewer vertices in X.

Proof . We apply induction on  |X|. If  |X| ≤  2 k −  1 we put  F =  ∅. So assume that  |X| ≥  2 k. Let  e  be an edge of  T  such that some component T 0  of  T − e  has at least  k  vertices in  X  and  |T 0|  is as small as possible.

As ∆( T )  ≤  3, the end of  e  in  T 0  has degree at most two in  T 0, so the minimality of  T 0  implies that  |X ∩ V ( T 0) | ≤  2 k −  1. Applying the induction hypothesis to  T − T 0  we complete the proof.

¤

Lemma 12.4.7.  Let G be a bipartite graph with bipartition ( A, B) ,

|A| =  a, |B| =  b, and let c ≤ a and d ≤ b be positive integers. Assume that G has at most ( a − c)( b − d) /d edges. Then there exist C ⊆ A and D ⊆ B such that |C| =  c and |D| =  d and C ∪ D is independent in G.

Proof . As  ||G|| ≤ ( a − c)( b − d) /d, fewer than  b − d  vertices in  B  have more than ( a − c) /d  neighbours in  A. Choose  D ⊆ B  so that  |D| =  d  and each vertex in  D  has at most ( a − c) /d  neighbours in  A. Then  D  sends a total of at most  a − c  edges to  A, so  A  has a subset  C  of  c  vertices without a neighbour in  D.

¤

Given a tree  T , call an  r-tuple ( x 1 , . . . , xr) of distinct vertices of  T

good  if, for every  j = 1 , . . . , r −  1, the  x good

j – xj+1 path in  T  contains none

r-tuple

of the other vertices in this  r-tuple.

Lemma 12.4.8.  Every tree T of order at least r( r −  1)  contains a good r-tuple of vertices.

Proof . Pick a vertex  x ∈ T . Then  T  is the union of its subpaths  xT y, where  y  ranges over its leaves. Hence unless one of these paths has at least  r  vertices,  T  has at least  |T |/( r −  1) >  r  leaves. Since any path of r  vertices and any set of  r  leaves gives rise to a good  r-tuple in  T , this proves the assertion.

¤

Our next lemma shows how to obtain a grid from two large systems

of paths that intersect in a particularly orderly way.

268

12. Minors, Trees, and WQO

Lemma 12.4.9.  Let d, r ≥  2  be integers such that d ≥ r 2 r+2 . Let G be a graph containing a set H of r 2  −  1  disjoint paths and a set V =  { V 1 , . . . , Vd } of d disjoint paths. Assume that every path in V meets every path in H, and that each path H ∈ H consists of d consecutive (vertex-disjoint) segments such that Vi meets H only in its ith segment, for every i = 1 , . . . , d (Fig. 12.4.3). Then G has an r × r grid minor.

H

. . .

}

H 1

. . .

H 2

. . .

.

{z  H

. .

Hr

. . .

. . .

|

V 1

Vd

Fig. 12.4.3.  Paths intersecting as in Lemma 12.4.9

Proof . For each  i = 1 , . . . , d, consider the graph with vertex set  H  in which two paths are adjacent whenever  Vi  contains a subpath between them that meets no other path in  H. Since  Vi  meets every path in  H, this Ti

is a connected graph; let  Ti  be a spanning tree in it. Since  |H| ≥ r( r −  1),

Lemma 12.4.8 implies that each of these  d ≥ r 2( r 2) r  trees  Ti  has a good r-tuple of vertices. Since there are no more than ( r 2) r  distinct  r-tuples H 1 , . . . , Hr

on  H, some  r 2 of the trees  Ti  have a common good  r-tuple ( H 1 , . . . , Hr).

I, ik

Let  I =  { i 1 , . . . , ir 2  }  be the index set of these trees (with  ij < ik  for H0

j < k) and put  H0 :=  { H 1 , . . . , Hr }.

Here is an informal description of how we construct our  r × r  grid.

Its ‘horizontal’ paths will be the paths  H 1 , . . . , Hr. Its ‘vertical’ paths will be pieced together edge by edge, as follows. The  r −  1 edges of the first vertical path will come from the first  r −  1 trees  Ti, trees with their index  i  among the first  r  elements of  I. More precisely, its ‘edge’ between Hj  and  Hj+1 will be the sequence of subpaths of  Vi (together with some j

connecting horizontal bits taken from paths in  H \ H0) induced by the edges of an  Hj– Hj+1 path in  Ti  that has no inner vertices in  H0; see j

Fig. 12.4.4. (This is why we need ( H 1 , . . . , Hr) to be a good  r-tuple in every tree  Ti.) Similarly, the  j th edge of the second vertical path will come from an  Hj– Hj+1 path in  Ti

, and so on.6 To merge these

r+ j

individual edges into  r  vertical paths, we then contract in each horizontal 6 Although we need only  r −  1 edges for each vertical path, we reserve  r  rather than just  r −  1 of the paths  Vi  for each vertical path to make the indexing more lucid.

The paths  Vi , V

, . . .  are left unused.

r

i 2 r

12.4 Tree-width and forbidden minors

269

Hj

Hj+1

H

H0

H00

P

H 1

H 3

H 2

The  Hj– Hj+1 path  P  in  Tij

Vij

ij th segment

P 0

H

Hj

P 0

H0

P 0

H00

Hj+1

Vij

The  Hj– Hj+1 path  P 0  in  G

Vi

V

V

V

1

ij

ir+1

i 2 r+1

H 1

Hj

P 0

Hj+1

Hr |

{z

} |

{z

} |

{z

}

contract

contract

contract

. . .

P 0  viewed as a (subdivided)  Hj– Hj+1 edge

Fig. 12.4.4.  An  Hj– Hj+1 path in  Ti  inducing segments of  V

j

ij

for the  j th edge of the grid’s first vertical path

270

12. Minors, Trees, and WQO

path the initial segment that meets the first  r  paths  Vi  with  i ∈ I, then contract the segment that meets the following  r  paths  Vi  with  i ∈ I, and so on.

Formally, we proceed as follows. Consider all  j, k ∈ {  1 , . . . , r }. (We shall think of the index  j  as counting the horizontal paths, and of the index  k  as counting the vertical paths of the grid to be constructed.) Let Hj  be the minimal subpath of  Hj  that contains the  i th segment of  Hj Hjk

k

ˆ

Hj

for all  i  with  i( k− 1) r < i ≤ ikr (put  i 0 := 0). Let ˆ

Hj  be obtained from

Hj  by first deleting any vertices following its  ir 2th segment and then vj

contracting every subpath  Hj  to one vertex  vj . Thus, ˆ

Hj =  vj . . . vj

k

k

k

1

r .

Given  j ∈ {  1 , . . . , r −  1  }  and  k ∈ {  1 , . . . , r }, we have to define a path  V j  that will form the subdivided ‘vertical edge’  vj vj+1. This path k

k k

will consist of segments of the path  Vi  together with some otherwise unused segments of paths from  H \ H0, for  i :=  i( k− 1) r+ j; recall that, by definition of ˆ

Hj  and ˆ

Hj+1, this  Vi  does indeed meet  Hj  and  Hj+1

precisely in vertices that were contracted into  vj  and  vj+1, respectively.

k

k

V j

To define  V j, consider an  Hj– Hj+1 path  P =  H

k

k

1  . . . Ht  in  Ti  that has

no inner vertices in  H0. (Thus,  H 1 =  Hj  and  Ht =  Hj+1.) Every edge  HsHs+1 of  P  corresponds to an  Hs– Hs+1 subpath of  Vi  that has no inner vertex on any path in  H. Together with (parts of) the  i th segments of  H 2 , . . . , Ht− 1, these subpaths of  Vi  form an  Hj– Hj+1 path P 0  in  G  that has no inner vertices on any of the paths  H 1 , . . . , Hr  and meets no path from  H  outside its  i th segment. Replacing the ends of  P 0

on  Hj  and  Hj+1 with  vj  and  vj+1, respectively, we obtain our desired k

k

path  V j  forming the  j th (subdivided) edge of the  k th ‘vertical’ path of k

our grid. Since the paths  P 0  are disjoint for different  i  and different pairs ( j, k) give rise to different  i, the paths  V j  are disjoint except for possible k

common ends  vj . Moreover, they have no inner vertices on any of the k

paths  H 1 , . . . , Hr, because none of these  Hj  is an inner vertex of any of the paths  P ⊆ Ti  used in the construction of  V j.

¤

k

Proof of Theorem 12.4.4. We are now ready to prove the following quantitative version of our theorem (which clearly implies it):

Let r, m >  0  be integers, and let G be a graph of tree-width at least r 4 m 2( r+2) . Then G contains either the r × r grid or Km as a minor.

Since  Kr 2 contains the  r × r  grid as a subgraph we may assume that c, k

2  ≤ m ≤ r 2. Put  c :=  r 4( r+2), and let  k :=  c 2( m) 2

. Then  c > 216 and

hence 2 m + 3  ≤ cm, so  G  has tree-width at least cm 2 =  cmk > (2 m + 3) k > ( m + 1)(2 k −  1) +  k −  1  , ( A, B)

enough for Lemma 12.4.5 to ensure that  G  contains a  k-mesh ( A, B) T

of order ( m + 1)(2 k −  1). Let  T ⊆ A  be a tree associated with the

12.4 Tree-width and forbidden minors

271

premesh ( A, B); then  X :=  V ( A ∩ B)  ⊆ V ( T ). By Lemma 12.4.6,  T  has X

|X|/(2 k −  1)  −  1 =  m  disjoint subtrees each containing at least  k  vertices of  X; let  A 1 , . . . , Am  be the vertex sets of these trees. By definition of a  A 1 , . . . , Am k-mesh,  B  contains for all 1  ≤ i < j ≤ m  a set  Pij  of  k  disjoint  Ai– Aj Pij

paths that have no inner vertices in  A. These sets  Pij  will shrink a little and be otherwise modified later in the proof, but they will always consist of ‘many’ disjoint  Ai– Aj  paths.

One option in our proof will be to find single paths  Pij ∈ Pij  that are disjoint for different pairs  ij  and thus link up the sets  Ai  to form a Km  minor of  G. If this fails, we shall instead exhibit two specific sets  Pij and  Ppq  such that many paths of  Pij  meet many paths of  Ppq, forming an  r × r  grid between them by Lemma 12.4.9.

Let us impose a linear ordering on the index pairs  ij  by fixing an

¡ ¢

arbitrary bijection  σ :  { ij |  1  ≤ i < j ≤ m } → {  0 ,  1 , . . . , m −  1  }. For σ

2

` = 0 ,  1 , . . .  in turn, we shall consider the pair  pq  with  σ( pq) =  `  and choose an  Ap– Aq  path  Ppq  that is disjoint from all previously selected such paths, i.e. from the paths  Pst  with  σ( st)  < `. At the same time, we shall replace all the ‘later’ sets  Pij—or what has become of them—by smaller sets containing only paths that are disjoint from  Ppq. Thus for each pair  ij, we shall define a sequence  Pij =  P 0  , P 1  , . . .  of smaller and ij

ij

smaller sets of paths, which eventually collapses to  P` =  { P

ij

ij }  when  `

has risen to  ` =  σ( ij).

¡ ¢

More formally, let  `∗ ≤ m  be the greatest integer such that, for

`∗

2

all 0  ≤ ` < `∗  and all 1 6  i < j  6  m, there exist sets  P`  satisfying the ij

following five conditions:

(i)  P`  is a non-empty set of disjoint  A

ij

i– Aj  paths in  B  that meet  A

only in their endpoints.

S

Whenever a set  P`  is defined, we shall write  H` :=

P`  for the union

ij

ij

ij

Hìj

of its paths.

(ii) If  σ( ij)  < `  then  P`  has exactly one element  P

ij

ij , and  Pij  does

Pij

not meet any path belonging to a set  P`st  with  ij 6=  st.

(iii) If  σ( ij) =  `, then  |P` | =  k/c 2 `.

ij

(iv) If  σ( ij)  > `, then  |P` | =  k/c 2 `+1.

ij

(v) If  ` =  σ( pq)  < σ( ij), then for every  e ∈ E( H` )  \ E( H`

ij

pq ) there are

no  k/c 2 `+1 disjoint  Ai– Aj  paths in the graph ( H` ∪

pq

H` )  − e.

ij

Note that, by (iv), the paths considered in (v) do exist in  H` . The ij

purpose of (v) is to force those paths to reuse edges from  H`pq  whenever possible, using new edges  e /

∈ H`pq  only if necessary. Note further

¡ ¢

that since  σ( ij)  < m  by definition of  σ, conditions (iii) and (iv) give 2

|P` | ≥ c 2 whenever  σ( ij)  ≥ `.

ij

¡ ¢

Clearly if  `∗ =  m  then by (i) and (ii) we have a (subdivided)  Km 2

¡ ¢

minor with branch sets  A

m

1 , . . . , Am  in  G. Suppose then that  `∗ <

.

2

272

12. Minors, Trees, and WQO

Let us show that  `∗ >  0. Let  pq :=  σ− 1(0) and put  P 0 pq :=  Ppq. To S

define  P 0 for  σ( ij)  >  0 put  H

P

ij

ij :=

ij , let  F ⊆ E( Hij )  \ E( H  0

pq )

be maximal such that ( H 0  ∪

pq

Hij)  − F  still contains  k/c  disjoint  Ai– Aj paths, and let  P 0 be such a set of paths. Since the vertices from  A ij

p ∪ Aq

have degree 1 in  H 0  ∪

pq

Hij  unless they also lie in  Ai ∪ Aj, these paths

have no inner vertices in  A. Our choices of  P 0 therefore satisfy (i)–(v) ij

for  ` = 0.

`

Having shown that  `∗ >  0, let us now consider  ` :=  `∗ −  1. Thus, conditions (i)–(v) are satisfied for  `  but cannot be satisfied for  ` + 1.

pq

Let  pq :=  σ− 1( `). If  P`pq  contains a path  P  that avoids a set  Qij  of some  |P` |/c  of the paths in  P`  for all  ij  with  σ( ij)  > `, then we can ij

ij

define  P`+1 for all  ij  as before (with a contradiction). Indeed, let  st :=

ij

S

σ− 1( ` + 1) and put  P`+1

Q

st

:=  Qst. For  σ( ij)  > ` + 1 write  Hij :=

ij ,

let  F ⊆ E( Hij)  \ E( H`+1

∪

st

) be maximal such that ( H`+1

st

Hij)  − F  still

contains at least  |P` |/c 2 disjoint  A

be such a set

ij

i– Aj  paths, and let  P `+1

ij

of paths. Setting  P`+1

}

pq

:=  { P }  and  P`+1 :=  P` =  { P `

for  σ( ij)  < `

ij

ij

ij

then gives us a family of sets  P`+1 that contradicts the maximality of  `∗.

ij

Thus for every path  P ∈ P`pq  there exists a pair  ij  with  σ( ij)  > `

such that  P  avoids fewer than  |P` |/c  of the paths in  P` . For some ij

ij

¡ ¢

P

d|P` | m e

pq /

of these  P  that pair  ij  will be the same; let  P  denote the set 2

¡ ¢

ij

of those  P , and keep  ij  fixed from now on. Note that  |P| ≥ |P` | m pq /

=

¡ ¢

2

c |P` |/ m  by (iii) and (iv).

ij

2

Let us use Lemma 12.4.7 to find sets  V ⊆ P ⊆ P`pq  and  H ⊆ Pìj such that

³

´

|V| > 1 |P|

≥ c |P` |

2

m 2

ij

|H| =  r 2

and every path in  V  meets every path in  H. We have to check that the bipartite graph with vertex sets  P  and  P`  in which  P ∈ P  is adjacent ij

to  Q ∈ P`  whenever  P ∩ Q =  ∅  does not have too many edges. Since ij

every  P ∈ P  has fewer than  |P` |/c  neighbours (by definition of  P), this ij

graph indeed has at most

|P||P` |

|

ij /c  6  |P ||P `

ij / 6 r 2 ±

6  b|P|/ 2 c |P` |

ij

2 r 2

¡

¢

6  b|P|/ 2 c |P` |

ij /r 2  −  1

¡

¢¡

¢±

=  |P| − d|P|/ 2 e |P` | −

ij

r 2

r 2

V, H

edges, as required. Hence,  V  and  H  exist as claimed.

Although all the (‘vertical’) paths in  V  meet all the (‘horizontal’) paths in  H, these paths do not necessarily intersect in such an orderly way as required for Lemma 12.4.9. In order to divide the paths from H  into segments, and to select paths from  V  meeting them only in the

12.4 Tree-width and forbidden minors

273

appropriate segments, we shall first pick a path  Q ∈ H  to serve as a yardstick: we shall divide  Q  into segments each meeting lots of paths from  V, select a ‘non-crossing’ subset  V 1 , . . . , Vd  of these vertical paths, one from each segment (which is the most delicate task; we shall need

condition (v) from the definition of the sets  P`  here), and finally divide ij

the other horizontal paths into the ‘induced’ segments, accommodating

one  Vn  each.

So let us pick a path  Q ∈ H, and put

Q

√

d :=  b c/mc =  br 2 r+4 /mc ≥ r 2 r+2 .

d

Note that  |V| > ( c/m 2) |P` | >  d 2 |P` |.

ij

ij

For  n = 1 ,  2 , . . . , d −  1 let  en  be the first edge of  Q (on its way from en

Ai  to  Aj) such that the initial component  Qn  of  Q − en  meets at least Qn

nd |P` |  different paths from  V, and such that  e

ij

n  is not an edge of  H `

pq .

As any two vertices of  Q  that lie on different paths from  V  are separated in  Q  by an edge not in  H`

|

pq , each of these  Qn  meets exactly  nd |P `

ij

paths from  V. Put  Q 0 :=  ∅  and  Qd :=  Q. Since  |V| ≥ d 2 |P` |, we have ij

thus divided  Q  into  d  consecutive disjoint segments  Q0n :=  Qn − Qn− 1

( n = 1 , . . . , d) each meeting at least  d |P` |  paths from  V.

ij

Q0 , . . . , Q0

1

d

For each  n = 1 , . . . , d −  1, Menger’s theorem (3.3.1) and conditions (iv) and (v) imply that  H` ∪

| −

pq

H`  has a set  S

1 vertices such

ij

n  of  |P `

ij

Sn

that ( H` ∪

pq

H` )  − e

ij

n − Sn  contains no path from  Ai  to  Aj . Let  S  denote S

the union of all these sets  Sn. Then  |S| < d |P` |, so each  Q0

ij

n  meets at

least one path  Vn ∈ V  that avoids  S (Fig. 12.4.5).

Vn

Q

Q0

e

e

e

1

1

en− 1  Q0n

n

d − 1  Q0d

}

. . .

. . .

}

P

P 0n

H {z

{z  r 2  −  1

|

|

Sn− 1

Sn

Vn

Ai

Aj

Fig. 12.4.5. Vn  meets every horizontal path but avoids  S

Clearly, each  Sn  consists of a choice of exactly one vertex  x  from every path  P ∈ P` \ { Q }. Denote the initial component of  P − x  by  P

ij

n,

put  P 0 :=  ∅  and  Pd :=  P , and let  P 0n :=  Pn − Pn− 1 for  n = 1 , . . . , d.

P 0 , . . . , P 0

1

d

The separation properties of the sets  Sn  now imply that  Vn ∩ P ⊆ P 0n  for

274

12. Minors, Trees, and WQO

n = 1 , . . . , d (and hence in particular that  P 0 6

n =  ∅, ie. that  Pn− 1  ⊂ Pn).

Indeed  Vn  cannot meet  Pn− 1, because  Pn− 1  ∪ Vn ∪ ( Q − Qn− 1) would then contain an  Ai– Aj  path in ( H` ∪

pq

H` )  − e

ij

n− 1  − Sn− 1, and likewise

(consider  Sn)  Vn  cannot meet  P − Pn. Thus for all  n = 1 , . . . , d, the path  Vn  meets every path  P ∈ H \ { Q }  precisely in its  n th segment  P 0n.

Applying Lemma 12.4.9 to the path systems  H \ { Q }  and  { V 1 , . . . , Vd }

now yields the desired grid minor.

¤

12.5 The graph minor theorem

Hereditary graph properties, those that are closed under taking minors, occur frequently in graph theory. Among the most natural examples

are the properties of being embeddable in some fixed surface, such as

planarity.

By Kuratowski’s theorem, planarity can be expressed by forbidding the minors  K 5 and  K 3 ,  3. This is a  good characterization  of planarity in the following sense. Suppose we wish to persuade someone that a certain graph is planar: this is easy (at least intuitively) if we can produce a drawing of the graph. But how do we persuade someone that a graph

is non-planar? By Kuratowski’s theorem, there is also an easy way to do that: we just have to exhibit an  M K 5 or  M K 3 ,  3 in our graph, as an easily checked ‘certificate’ for non-planarity. Our simple Proposition

12.4.2 is another example of a good characterization: if a graph has tree width  <  3, we can prove this by exhibiting a suitable tree-decomposition; if not, we can produce an  M K 4 as evidence.

Theorems that characterize a hereditary property  P  by a set  H

of forbidden minors are doubtless among the most attractive results in

graph theory. As we saw in the proof of Proposition 12.4.1, there is always some such characterization: that where  H  is the complement  P

of  P. However, one naturally seeks to make  H  as small as possible. And as it turns out, there is indeed a unique smallest such set  H: the set HP :=  { H | H  is 4-minimal in  P }

satisfies  P = Forb4( H) and is contained in every other such set  H.

Proposition 12.5.1.  P = Forb4( HP) , and HP ⊆ H for every set H

with P = Forb4( H) .

¤

Clearly, the elements of  HP  are incomparable under the minor relation 4. Now the  graph minor theorem  of Robertson & Seymour says that any set of 4-incomparable graphs must be finite:

12.5 The graph minor theorem

275

Theorem 12.5.2. (Graph Minor Theorem; Robertson & Seymour)

The finite graphs are well-quasi-ordered by the minor relation  4 .

So every  HP  is finite, i.e. every hereditary graph property can be represented by finitely many forbidden minors:

Corollary 12.5.3.  Every graph property that is closed under taking minors can be expressed as  Forb4( H)  with finite H.

¤

As a special case of Corollary 12.5.3 we have, at least in principle,

a Kuratowski-type theorem for every surface:

Corollary 12.5.4.  For every surface S there exists a finite set of graphs H 1 , . . . , Hn such that  Forb4( H 1 , . . . , Hn)  contains precisely the graphs not embeddable in S.

The minimal set of forbidden minors has been determined explicitly

for only one surface other than the sphere: for the projective plane it is known to consist of 35 forbidden minors. It is not difficult to show that the number of forbidden minors grows rapidly with the genus of the surface (Exercise 34).

The complete proof of the graph minor theorem would fill a book

or two. For all its complexity in detail, however, its basic idea is easy to grasp. We have to show that every infinite sequence

G 0 , G 1 , G 2 , . . .

of finite graphs contains a good pair: two graphs  Gi  4  Gj  with  i < j.

We may assume that  G 0  6 4  Gi  for all  i > 1, since  G 0 forms a good pair with any graph  Gi  of which it is a minor. Thus all the graphs  G 1 , G 2 , . . .

lie in Forb4( G 0), and we may use the structure common to these graphs in our search for a good pair.

We have already seen how this works when  G 0 is planar: then the graphs in Forb4( G 0) have bounded tree-width (Theorem 12.4.3) and are therefore well-quasi-ordered by Theorem 12.3.7. In general, we need only consider the cases of  G 0 =  Kn: since  G 0 4  Kn  for  n :=  |G 0 |, we may assume that  Kn 6 4  Gi  for all  i > 1.

The proof now follows the same lines as above: again the graphs in

Forb4( Kn) can be characterized by their tree-decompositions, and again their tree structure helps, as in Kruskal’s theorem, with the proof that they are well-quasi-ordered. The parts in these tree-decompositions are no longer restricted in terms of order now, but they are constrained in more subtle structural terms. Roughly speaking, for every  n  there exists a finite set  S  of closed surfaces such that every graph without a  Kn  minor has a simplicial tree-decomposition into parts each ‘nearly’ embedding in

276

12. Minors, Trees, and WQO

one of the surfaces  S ∈ S. (The ‘nearly’ hides a measure of disorderliness that depends on  n  but not on the graph to be embedded.) By a generalization of Theorem 12.3.7—and hence of Kruskal’s theorem—it now suffices, essentially, to prove that the set of all the parts in these tree-decompositions is well-quasi-ordered: then the graphs decomposing into

these parts are well-quasi-ordered, too. Since  S  is finite, every infinite sequence of such parts has an infinite subsequence whose members are

all (nearly) embeddable in the same surface  S ∈ S. Thus all we have to show is that, given any closed surface  S, all the graphs embeddable in S  are well-quasi-ordered by the minor relation.

This is shown by induction on the genus of  S (more precisely,

on 2  − χ( S), where  χ( S) denotes the Euler characteristic of  S) using the same approach as before: if  H 0 , H 1 , H 2 , . . .  is an infinite sequence of graphs embeddable in  S, we may assume that none of the graphs H 1 , H 2 , . . .  contains  H 0 as a minor. If  S =  S 2 we are back in the case that  H 0 is planar, so the induction starts. For the induction step we now assume that  S 6=  S 2. Again, the exclusion of  H 0 as a minor constrains the structure of the graphs  H 1 , H 2 , . . . , this time topologically: each  Hi  with  i > 1 has an embedding in  S  which meets some non-contractible closed curve  Ci ⊆ S  in no more than a bounded number of vertices (and no edges), say in  Xi ⊆ V ( Hi). (The bound on  |Xi|

depends on  H 0, but not on  Hi.) Cutting along  Ci, and sewing a disc on to each of the one or two closed boundary curves arising from the cut,

we obtain one or two new closed surfaces of larger Euler characteristic.

If the cut produces only one new surface  Si, then our embedding of Hi − Xi  still counts as a near-embedding of  Hi  in  Si (since  Xi  is small).

If this happens for infinitely many  i, then infinitely many of the surfaces  Si  are also the same, and the induction hypothesis gives us a good pair among the corresponding graphs  Hi. On the other hand, if we get two surfaces  S0  and  S00  for infinitely many  i (without loss of generality i

i

the same two surfaces), then  Hi  decomposes accordingly into subgraphs H0  and  H00  embedded in these surfaces, with  V ( H0 ∩ H00) =  X

i

i

i

i

i. The

set of all these subgraphs taken together is again well-quasi-ordered by the induction hypothesis, and hence so are the pairs ( H0, H00) by Lem-i

i

ma 12.1.3. Using a sharpening of the lemma that takes into account

not only the graphs  H0  and  H00  themselves but also how  X

i

i

i  lies in-

side them, we finally obtain indices  i, j  not only with  H0  4  H0  and i

j

H00  4  H00, but also such that these minor embeddings extend to the de-i

j

sired minor embedding of  Hi  in  Hj—completing the proof of the minor

theorem.

In addition to its impact on ‘pure’ graph theory, the graph minor

theorem has had far-reaching algorithmic consequences. Using their tree structure theorem for the graphs in Forb4( Kn), Robertson & Seymour have shown that testing for any fixed minor is ‘fast’: for every graph  H

12.5 The graph minor theorem

277

there is a polynomial-time algorithm7 that decides whether or not the

input graph contains  H  as a minor. By the minor theorem, then, every hereditary graph property  P  can be decided in polynomial (even cubic) time: if  H 1 , . . . , Hk  are the corresponding minimal forbidden minors, then testing a graph  G  for membership in  P  reduces to testing the  k assertions  Hi  4  G!

The following example gives an indication of how deeply this algo-

rithmic corollary affects the complexity theory of graph algorithms. Let us call a graph  knotless  if it can be embedded in R3 so that none of its cycles forms a non-trivial knot. Before the graph minor theorem, it was an open problem whether knotlessness is decidable, that is, whether  any algorithm exists (no matter how slow) that decides for any given graph

whether or not that graph is knotless. To this day, no such algorithm

is known. The property of knotlessness, however, is easily ‘seen’ to be hereditary: contracting an edge of a graph embedded in 3-space will not create a knot where none had been before. Hence, by the minor theorem,

there  exists  an algorithm that decides knotlessness—even in polynomial (cubic) time!

However spectacular such unexpected solutions to long-standing

problems may be, viewing the graph minor theorem merely in terms of its corollaries will not do it justice. At least as important are the techniques developed for its proof, the various ways in which minors are handled or constructed. Most of these have not even been touched upon

here, yet they seem set to influence the development of graph theory for many years to come.

Exercises

1.  −  Let 6 be a quasi-ordering on a set  X. Call two elements  x, y ∈ X

equivalent  if both  x  6  y  and  y  6  x. Show that this is indeed an equivalence relation on  X, and that 6 induces a partial ordering on the set of equivalence classes.

2.

Let ( A,  6) be a quasi-ordering. For subsets  X ⊆ A  write Forb6( X) :=  { a ∈ A | a 6>  x  for all  x ∈ X } .

Show that 6 is a well-quasi-ordering on  A  if and only if every subset B ⊆ A  that is closed under > (i.e. such that  x  6  y ∈ B ⇒ x ∈ B) can be written as  B = Forb6( X) with finite  X.

3.

Prove Proposition 12.1.1 and Corollary 12.1.2 directly, without using

Ramsey’s theorem.

7 indeed a cubic one—although with a typically enormous constant depending on  H

278

12. Minors, Trees, and WQO

4.

Given a quasi-ordering ( X,  6) and subsets  A, B ⊆ X, write  A  6 0 B  if there exists an  order preserving  injection  f :  A → B  with  a  6  f ( a) for all  a ∈ A. Does Lemma 12.1.3 still hold if the quasi-ordering considered for [ X] <ω  is 6 0?

5.  −  Show that the relation 6 between rooted trees defined in the text is indeed a quasi-ordering.

6.

Show that the finite trees are not well-quasi-ordered by the subgraph

relation.

7.

The last step of the proof of Kruskal’s theorem considers a ‘topological’

embedding of  Tm  in  Tn  that maps the root of  Tm  to the root of  Tn.

Suppose we assume inductively that the trees of  Am  are embedded in the trees of  An  in the same way, with roots mapped to roots. We thus seem to obtain a proof that the finite rooted trees are well-quasi-ordered by the subgraph relation, even with roots mapped to roots. Where is

the error?

8.+ Show that the finite graphs are not well-quasi-ordered by the topological minor relation.

9.+ Given  k ∈  N, is the class  { G | G 6⊇ P k }  well-quasi-ordered by the subgraph relation?

10.

Show that a graph has tree-width at most 1 if and only if it is a forest.

11.

Let  G  be a graph,  T  a set, and ( Vt) t∈T  a family of subsets of  V ( G) satisfying (T1) and (T2) from the definition of a tree-decomposition. Show

that there exists a tree on  T  that makes (T3) true if and only if there exists an enumeration  t 1 , . . . , tn  of  T  such that for every  k = 2 , . . . , n S

there is a  j < k  satisfying  Vt ∩

V

⊆ V .

k

i<k

ti

tj

(The new condition tends to be more convenient to check than (T3).

It can help, for example, with the construction of a tree-decomposition into a given set of parts.)

12.

Prove the following converse of Lemma 12.3.1: if ( T, V) satisfies condition (T1) and the statement of the lemma, then ( T, V) is a tree-decomposition of  G.

13.

Can the tree-width of a subdivision of a graph  G  be smaller than tw( G)?

Can it be larger?

14.

Let ( T, ( Vt) t∈T ) be a tree-decomposition of a graph  G. For each vertex v ∈ G, set  Tv :=  { t ∈ T | v ∈ Vt }. Show that  Tv  is always connected in  T . More generally, for which subsets  U ⊆ V ( G) is the set

{ t ∈ T | Vt ∩ U 6=  ∅ }  always connected in  T (i.e. for all tree-decompositions)?

15.  −  Show that the tree-width of a graph is one less than its bramble number.

16.

Apply Theorem 12.3.9 to show that the  k × k  grid has tree-width at least  k, and find a tree-decomposition of width exactly  k.

Exercises

279

17.

Let  B  be a maximum-order bramble in a graph  G. Show that every minimum-width tree-decomposition of  G  has a unique part covering  B.

18.+ In the second half of the proof of Theorem 12.3.9, let  H0  be the union of  H  and the paths  P 1 , . . . , P`, let  H00  be the graph obtained from  H0

by contracting each  Pi, and let ( T, ( W 00

t )

) be the tree-decomposi-

t∈T

tion induced on  H00 (as in Lemma 12.3.3) by the decomposition that ( T, ( Vt) t∈T ) induces on  H0. Is this, after the obvious identification of H00  with  H, the same decomposition as the one used in the proof, i.e.

is  W 00

t

=  Wt  for all  t ∈ T ?

19.

Show that any graph with a simplicial tree-decomposition into  k-

colourable parts is itself  k-colourable.

20.

Let  H  be a set of graphs, and let  G  be constructed recursively from elements of  H  by pasting along complete subgraphs. Show that  G  has a simplicial tree-decomposition into elements of  H.

21.

Given a tree-decomposition ( T, ( Vt) t∈T ) of  G  and  t ∈ T , let  Ht  denote the graph obtained from  G [  Vt ] by adding all the edges  xy  such that x, y ∈ Vt ∩ Vt0  for some neighbour  t0  of  t  in  T ; the graphs  Ht  are called the  torsos  of this tree-decomposition. Show that  G  has no  K 5 minor if and only if  G  has a tree-decomposition in which every torso is either planar or a copy of the Wagner graph  W (Fig. 8.3.1).

22.+ Call a graph  irreducible  if it is not separated by any complete subgraph.

Every (finite) graph  G  can be decomposed into irreducible induced subgraphs, as follows. If  G  has a separating complete subgraph  S, then decompose  G  into proper induced subgraphs  G0  and  G00  with  G =

G0 ∪ G00  and  G0 ∩ G00 =  S. Then decompose  G0  and  G00  in the same way, and so on, until all the graphs obtained are irreducible. By Exercise 20,

G  has a simplicial tree-decomposition into these irreducible subgraphs.

Show that they are uniquely determined if the complete separators were

all chosen minimal.

23.+ If  F  is a family of sets, then the graph  G  on  F  with  XY ∈ E( G)  ⇔

X ∩ Y 6=  ∅  is called the  intersection graph  of  F. Show that a graph is chordal if and only if it is isomorphic to the intersection graph of a family of (vertex sets of) subtrees of a tree.

24.

A tree-decomposition of a graph is called a  path-decomposition  if its decomposition tree is a path. Show that a graph has a path-decomposition into complete graphs if and only if it is isomorphic to an interval graph. (Interval graphs are defined in Ex. 37, Ch. 5.)

25.

(continued)

The  path-width  pw( G) of a graph  G  is the least width of a path-decomposition of  G. Prove the following analogue of Corollary 12.3.12 for path-width: every graph  G  satisfies pw( G) = min  ω( H)  −  1, where the minimum is taken over all interval graphs  H  containing  G.

26.+ Do trees have unbounded path-width?

280

12. Minors, Trees, and WQO

27.

Let  P  be a hereditary graph property. Show that strengthening the notion of a minor (for example, to that of topological minor) increases the set of forbidden minors required to characterize  P.

28.

Deduce from the minor theorem that every hereditary property can be expressed by forbidding finitely many topological minors. Is the same

true for every property that is closed under taking topological minors?

29.

Show that every horizontal path in the  k × k  grid is externally  k-

connected in that grid.

30.+ Show that the tree-width of a graph is large if and only if it contains a large externally  k-connected set of vertices, with  k  large. For example, show that graphs of tree-width  < k  contain no externally ( k + 1)-

connected set of 3 k  vertices, and that graphs containing no externally ( k + 1)-connected set of 3 k  vertices have tree-width  <  4 k.

31.+ (continued)

Find an N  →  N2 function  k 7→ ( h, `) such that every graph with an externally  `-connected set of  h  vertices contains a bramble of order at least  k. Deduce the weakening of Theorem 12.3.9 that, given  k, every graph of large enough tree-width contains a bramble of order at least  k.

32.

Without using the minor theorem, show that the chromatic number of the graphs in any 4-antichain is bounded.

33.

Seymour’s  self-minor conjecture  asserts that ‘every countably infinite graph is a proper minor of itself’. Make this assertion precise, and

deduce the minor theorem from it.

34.

Given an orientable surface  S  of genus  g, find a lower bound in terms of  g  for the number of forbidden minors needed to characterize embeddability in  S.

(Hint. The smallest genus of an orientable surface in which a given

graph can be embedded is called the (orientable)  genus  of that graph.

Use the theorem that the genus of a graph is equal to the sum of the

genera of its blocks.)

Notes

Kruskal’s theorem on the well-quasi-ordering of finite trees was first published in J.A. Kruskal, Well-quasi ordering, the tree theorem, and Vászonyi’s conjecture,  Trans. Amer. Math. Soc. 95 (1960), 210–225. Our proof is due to Nash-Williams, who introduced the versatile proof technique of choosing a ‘minimal bad sequence’. This technique was also used in our proof of Higman’s Lemma

12.1.3.

Nash-Williams generalized Kruskal’s theorem to infinite graphs. This extension is much more difficult than the finite case; it is one of the deepest theorems in infinite graph theory. The general graph minor theorem becomes false for arbitrary infinite graphs, as shown by R. Thomas, A counterexample to ‘Wagner’s conjecture’ for infinite graphs,  Math. Proc. Camb. Phil. Soc. 103

Notes

281

(1988), 55–57. Whether or not the minor theorem extends to countable graphs remains an open problem.

The notions of tree-decomposition and tree-width were first introduced

(under different names) by R. Halin,  S-functions for graphs,  J. Geometry 8

(1976), 171–186. Among other things, Halin showed that grids can have arbitrarily large tree-width. Robertson & Seymour reintroduced the two concepts, apparently unaware of Halin’s paper, with direct reference to K. Wagner, Über eine Eigenschaft der ebenen Komplexe,  Math. Ann. 114 (1937), 570–

590.

(This is the classic paper that introduced simplicial tree-decomposi-

tions to prove Theorem 8.3.4; cf. Exercise 21.) Simplicial tree-decompositions are treated in depth in R. Diestel,  Graph Decompositions, Oxford University Press 1990.

Robertson & Seymour themselves usually refer to the graph minor theorem as  Wagner’s conjecture. It seems that Wagner did indeed discuss this problem in the 1960s with his then students Halin and Mader.

However,

Wagner apparently never conjectured a positive solution; he certainly rejected any credit for the ‘conjecture’ when it had been proved.

Robertson & Seymour’s proof of the graph minor theorem is given in the numbers IV–VII, IX–XII and XIV–XX of their series of over 20 papers under the common title of  Graph Minors, which has been appearing in the Journal of Combinatorial Theory, Series B, since 1983. Of their theorems cited in this chapter, Theorem 12.3.7 is from Graph Minors IV, while Theorems

12.4.3 and 12.4.4 are from Graph Minors V. Our short proof of these latter theorems is from R. Diestel, K.Yu. Gorbunov, T.R. Jensen & C. Thomassen, Highly connected sets and the excluded grid theorem,  J. Combin. Theory B

75 (1999), 61–73.

Theorem 12.3.9 is due to P.D. Seymour & R. Thomas, Graph searching and a min-max theorem for tree-width,  J. Combin. Theory B 58 (1993), 22–33. Our proof is a simplification of the original proof. B.A. Reed gives an instructive introductory survey on tree-width and graph minors, including some algorithmic aspects, in (R.A. Bailey, ed)  Surveys in Combinatorics 1997 , Cambridge University Press 1997, 87–162. Reed also introduced the term ‘bramble’; in Seymour & Thomas’s paper, brambles are called ‘screens’.

The obstructions to small tree-width actually used in the proof of the

graph minor theorem are not brambles but so-called  tangles. Tangles are more powerful than brambles and well worth studying. See Graph Minors X

or Reed’s survey for an introduction to tangles and their relation to brambles and tree-decompositions.

Theorem 12.3.10 is due to R. Thomas, A Menger-like property of tree-width; the finite case,  J. Combin. Theory B 48 (1990), 67–76.

As a forerunner to Theorem 12.4.3, Robertson & Seymour proved its following analogue for path-width (Graph Minors I): excluding a graph  H  as a minor bounds the path-width of a graph if and only if  H  is a forest. A short proof of this result, with optimal bounds, can be found in the first edition of this book, or in R. Diestel, Graph Minors I: a short proof of the path width theorem,  Combinatorics, Probability and Computing 4 (1995), 27–30.

The 35 minimal forbidden minors for graphs to be embedded in the pro-

jective plane were determined by D. Archdeacon, A Kuratowski theorem for the projective plane,  J. Graph Theory 5 (1981), 243–246. An upper bound for

282

12. Minors, Trees, and WQO

the number of forbidden minors needed for an arbitrary closed surface is given in P.D. Seymour, A bound on the excluded minors for a surface,  J. Combin.

Theory B (to appear). B. Mohar, Embedding graphs in an arbitrary surface in linear time,  Proc. 28th Ann. ACM STOC (Philadelphia 1996), 392–397, has developed a set of algorithms, one for each surface, that decide embeddability in that surface in linear time. As a corollary, Mohar obtains an independent and constructive proof of the ‘generalized Kuratowski theorem’, Corollary

12.5.4. Another independent and short proof of this corollary, which builds on

Theorem 12.4.3 and Graph Minors IV but on no other papers of the Graph Minors series, was found by C. Thomassen, A simpler proof of the excluded minor theorem for higher surfaces,  J. Combin. Theory B 70 (1997), 306–311.

A survey of the classical forbidden minor theorems is given in Chapter 6.1 of R. Diestel,  Graph Decompositions, Oxford University Press 1990. More recent developments are surveyed in R. Thomas, Recent excluded minor theorems, in (J.D. Lamb & D.A. Preece, eds)  Surveys in Combinatorics 1999 , Cambridge University Press 1999, 201–222.

For every graph  X, Graph Minors XIII gives an explicit algorithm that decides in cubic time for every input graph  G  whether  X  4  G. The constants in the cubic polynomials bounding the running time of these algorithms depend on  X  but are constructively bounded from above. For an overview of the algorithmic implications of the Graph Minors series, see Johnson’s NP-completeness column in  J. Algorithms 8 (1987), 285–303.

The concept of a ‘good characterization’ of a graph property was first

suggested by J. Edmonds, Minimum partition of a matroid into independent subsets,  J. Research of the National Bureau of Standards (B) 69 (1965) 67–72.

In the language of complexity theory, a characterization is  good  if it specifies two assertions about a graph such that, given any graph  G, the first assertion holds for  G  if and only if the second fails, and such that each assertion, if true for  G, provides a certificate for its truth. Thus every good characterization has the corollary that the decision problem corresponding to the property it characterizes lies in NP  ∩  co-NP.

Hints for all the

Exercises

Caveat. These hints are intended to set on the right track anyone who has already spent some time over an exercise but somehow failed to make much progress. They are not designed to be particularly intelligible

without such an initial attempt, and they will rarely spoil the fun by

giving away the key idea. They may, however, narrow ones mind by

focusing on what is just one of several possible ways to think about a

problem . . .

Hints for Chapter 1

1.  −  How many edges are there at each vertex?

2.

Average degree and edges: consider the vertex degrees. Diameter: how

do you determine the distance between two vertices from the corre-

sponding 0–1 sequences? Girth: does the graph have a cycle of length 3?

Circumference: partition the  d-dimensional cube into cubes of lower dimension and use induction.

3.

Consider how the path intersects  C. Where can you see cycles, and can they all be short?

4.  −  Can you find graphs for which Proposition 1.3.2 holds with equality?

5.

Estimate the distances within  G  as seen from a central vertex.

6.+ Consider the cases  d = 2 and  d >  2 separately. For  d >  2, give a sharper bound on  |Di|  for  i >  0 than the one used in the proof of Proposition

1.3.3.

7.  −  Assume the contrary, and derive a contradiction.

8.  −  Find two vertices that are linked by two independent paths.

284

Hints for all the exercises

9.

(i) Straightforward from the definitions.

(ii) Prove  κ >  n  by induction on  n: partition the  n-dimensional cube into cubes of lower dimension, and show inductively that the deletion

of  < n  vertices leaves a connected subgraph.

10.

For the first inequality, consider the endvertices of a set of  λ( G) edges whose deletion disconnects  G. Use the definition of  λ( G) to show the second inequality.

11.  −  Try to find counterexamples for  k = 1.

12.

Rephrase (i) and (ii) as statements about the existence of two N  →  N

functions. To show the equivalence, express each of these functions in

terms of the other. Show that (iii) may hold even if (i) and (ii) do not, and strengthen (iii) to remedy this.

13.+ Try to imitate the proof assuming  ε( G) > 2 k  instead of condition (ii).

Why does this fail, and why does condition (ii) remedy the problem?

14.

Show (i)  ⇒ (ii)  ⇒ (iii)  ⇒ (iv)  ⇒ (i) from the definitions of the relevant concepts.

15.

Consider paths emanating from a vertex of maximum degree.

16.

Theorem 1.5.1.

17.

Induction.

18.

The easiest solution is to apply induction on  |T |. What kind of vertex of  T  will be best to delete in the induction step?

19.

Induction on  |T |  is a possibility, but not the only one.

20.

Count the edges.

21.

Show that if a graph contains any odd cycle at all it also contains an

induced one.

22.

Apply Proposition 1.2.2. Split the subgraph thus found into two sides so that every vertex has many neighbours on the opposite side.

23.

Try to carry the proof for finite graphs over to the infinite case. Where does it fail?

24.  −  Use Proposition 1.9.2.

25.

Why do  all  the cuts  E( v) generate the cut space? Will they still do so if we omit one of them? Or even two?

26.

Start with the case that the graph considered is a cycle.

27.

Induction on  |F  r  E( T ) |  for given  F ∈ C( G).

28.

Induction on  |D ∩ E( T ) |  for a given cut  D.

29.

Apply Theorem 1.9.6.

Hints for Chapter 2

285

Hints for Chapter 2

1.

Compare the given matching with a matching of maximum cardinality.

2.

Augmenting paths.

3.

If you have  S $  S0 ⊆ A  with  |S| =  |N ( S) |  in the finite case, the marriage condition ensures that  N ( S) $  N ( S0): increasing  S  makes more neighbours available. Use the fact that this fails when  S  is infinite.

4.

Apply the marriage theorem.

5.

Construct a bipartite graph in which  A  is one side, and the other side consists of a suitable number of copies of the sets  Ai. Define the edge set of the graph so that the desired result can be derived from the

marriage theorem.

6.+ Construct chains in the power set lattice of  X  as follows. For each k < n/ 2, use the marriage theorem to find a 1–1 map  ϕ  from the set  A of  k-subsets to the set  B  of ( k + 1)-subsets of  X  such that  Y ⊆ ϕ( Y ) for all  Y ∈ A.

7.

Decide where the leaves should go: in factor-critical components or

in  S?

8.

Distinguish between the cases of  |S| ≤  1 and  |S| ≥  2.

9.

The case  S =  ∅  is easy. In the other case, look for a vertex that meets every maximum-cardinality matching. What are the consequences of

this for the other vertices?

10.

For the ‘if’ direction consider the graph suggested in the hint: does it have a 1-factor? If not, then consider the set of vertices supplied by

Tutte’s 1-factor theorem. For an alternative solution, apply the remarks on maximum-cardinality matchings which follow Theorem 2.2.3.

11.  − Corollary 2.2.2.

12.

Let  G  be a bipartite graph that satisfies the marriage condition, with bipartition ( A, B) say. Reduce the problem to the case of  |A| =  |B|. To verify the premise of Tutte’s theorem for a given set  S ⊆ V ( G), bound

|S|  from below in terms of the number of components of  G − S  that contain more vertices from  A  than from  B  and vice versa.

13.  −  Consider any smallest path cover.

14.

Direct all the edges from  A  to  B.

15.  −  Dilworth.

16.

Start with the set of minimal elements in  P .

17.

Think of the elements of  A  as being smaller than their neighbours in  B.

18.+ Let ( x, y) 6 ( x0, y0) if and only if  x  6  x0  and  y  6  y0.

286

Hints for all the exercises

Hints for Chapter 3

1.  −  Recall the definitions of ‘separate’ and ‘component’.

2.

Describe in words what the picture suggests.

3.

Use Exercise 1 to answer the first question. The second requires an elementary calculation, which the figure may already suggest.

4.

Only the first part needs arguing; the second then follows by symmetry.

So suppose a component of  G − X  is not met by  X0, and refer to

Exercise 1. Where does  X0  lie? Are all our assumptions about  X0

consistent?

5.  −  How can a block fail to be a maximal 2-connected subgraph? And what else follows then?

6.

Deduce the connectedness of the block graph from that of the graph

itself, and its acyclicity from the maximality of each block.

7.

Prove the statement inductively using Proposition 3.1.2. Alternatively, choose a cycle through one of the two vertices and with minimum

distance from the other vertex. Show that this distance cannot be

positive.

8.

Belonging to the same block is an equivalence relation on the edge set; see Exercise 5.

9.

Induction along Proposition 3.1.2.

10.

Assuming that  G/xy  is not 3-connected, distinguish the cases when  vxy lies inside or outside a separating set of at most 2 vertices.

11.

(i) Consider the edges incident with a smaller separator.

(ii) Induction shows that all the graphs obtained by the construction are cubic and 3-connected. For the converse, consider a maximal subgraph

T H ⊆ G  such that  H  is constructible as stated; then show that  H =  G.

12.  −  Can  any  choice of  X  and  P  as suggested by Menger’s theorem fail?

13.

Choose the disjoint  A– B  paths in  L( G) minimal.

14.

Consider a longest cycle  C. How are the other vertices joined to  C?

15.

Consider a cycle through as many of the  k  given vertices as possible.

If one them is missed, can you re-route the cycle through it?

16.

Consider the graph of the hint. Show that any subset of its vertices

that meets all  H-paths (but not  H) corresponds to a similar subset of  E( G) r  E( H). What does a pair of independent  H-paths in the auxiliary graph correspond to in  G?

17.  −  How many paths can a single  K 2 m+1 accomodate?

18.

Choose suitable degrees for the vertices in  B.

19.+ Let  H  be the (edgeless) graph on the new vertices. Consider the sets X  and  F  that Mader’s theorem provides if  G0  does not contain  |G|/ 2

independent  H-paths. If  G  has no 1-factor, use these to find a suitable set that can play the role of  S  in Tutte’s theorem.

20.

Think small.

21.  −  If two vertices  s, t  are separated by fewer than 2 k −  1 vertices, extend

{ s }  and  { t }  to  k-sets  S  and  T  showing that  G  is not  k-linked.

Hints for Chapter 4

287

Hints for Chapter 4

1.

Embed the vertices inductively. Where should you  not  put the new vertex?

2.  − Figure 1.6.2.

3.  −  Make the given graph connected.

4.

This is a generalization of Corollary 4.2.8.

5.

Theorem 3.5.4.

6.

Imitate the proof of Corollary 4.2.8.

7.

Proposition 4.2.10.

8.  −  Express the difference between the two drawings as a formal statement about vertices, faces, and the incidences between them.

9.

Combinatorially: use the definition. Topologically: express the relative position of the short (respectively, the long) branches of  G0  formally as a property of  G0  which any topological ismorphism would preserve but G  lacks.

10.  −  Reflexivity, symmetry, transitivity.

11.

Look for a graph whose drawings all look the same, but which admits

an automorphism that does not extend to a homeomorphism of the

plane. Interpret this automorphism as  σ 2  ◦ σ− 1.

1

12.+ Star-shape: every inner face contains a point that sees the entire face boundary.

13.

Work with plane rather than planar graphs.

14.

(i) The set  X  may be infinite.

(ii) If  Y  is a  T X, then every  T Y  is also a  T X.

15.  −  By the next exercise, any counterexample can be disconnected by at most two vertices.

16.

Incorporate the extra condition into the induction hypothesis of the

proof. It may help to disallow polygons with 180 degree angles.

17.

Number of edges.

18.

Use that maximal planar graphs are 3-connected, and that the neigh-

bours of each vertex induce a cycle.

19.

If  G =  G 1  ∪ G 2 with  G 1  ∩ G 2 =  K 2, we have a problem. This will go away if we embed a little more than necessary.

20.

Use a suitable modification of the given graph  G  to simulate outerplanarity.

21.

Use the fact that  C( G) is the direct sum of  C( G 1) and  C( G 2).

22.+ Euler.

23.

The face boundaries generate  C( G).

24.  −  Which are the faces that  e∗ (viewed as a polygonal arc) can meet?

25.  −  How many vertices does it have?

26.  −  Join two given vertices of the dual by a straight line, and use this to find a path between them in the dual graph.

288

Hints for all the exercises

27.+ To show existence, define the required bijections  F → V ∗,  E → E∗, V → F ∗  successively in this order, while at the same time constructing  G∗. Show that connectedness is necessary to ensure that these three functions can all be made bijective.

28.

Solve the previous exercise first.

29.

Use the bijections that come with the two duals to define the desired

isomorphism and to prove that it is combinatorial.

30.

Apply Menger’s theorem and Proposition 4.6.1. For (iii), consider a 4-connected graph with six vertices.

31.

Apply induction on  n, starting with part (i) of the previous exercise.

32.

Theorem 1.9.5.

33.

For the forward implication, consider  G0 :=  G∗. For the converse, apply a suitable planarity criterion.

Hints for Chapter 5

1.  −  Duality.

2.  −  Whenever more than three countries have some point in common, apply a small local change to the map where this happens.

3.

Where does the five colour proof use the fact that  v  has no more neighbours than there are colours?

4.

How can the colourings of different blocks interfere with each other?

5.  −  Use a colouring of  G  to derive a suitable ordering.

6.

Consider how the removal of certain edges may lead the greedy algo-

rithm to use more colours.

7.

Describe more precisely how to implement this alternative algorithm.

Then, where is the difference to the traditional greedy algorithm?

8.

Compare the number of edges in a subgraph  H  as in 5.2.2 with the number  m  of edges in  G.

9.

To find  f , consider a given graph of small colouring number and partition it inductively into a small number of forest. For  g, use Proposition 5.2.2 and the easy direction of Theorem 3.5.4.

10.  −  Remove vertices successively until the graph becomes critically  k-

chromatic. What can you say about the degree of any vertex that

remains?

11.

Proposition 1.6.1.

12.+ Modify colourings of the two sides of a hypothetical cut of fewer than k −  1 edges so that they combine to a ( k −  1)-colouring of the entire graph (with a contradiction).

13.

Proposition 1.3.1.

14.  −  For which graphs with large maximum degree does Proposition 5.2.2

give a particularly small upper bound?

Hints for Chapter 5

289

15.+ (i) How will  v 1 and  v 2 be coloured, and how  vn?

(ii) Consider the subgraph induced by the neighbours of  vn.

16.

For the induction start, explicitly calculate  PG( k) for  |G| =  n  and kGk = 0.

17.+ Derive from the polynomial the number of edges and the number of

components of  G; see the previous exercise.

18.

Imitate the proof of Theorem 5.2.5.

19.  − Kn,n.

20.

How are edge colourings related to matchings?

21.

Construct a bipartite ∆( G)-regular graph that contains  G  as subgraph.

It may be necessary to add some vertices.

22.+ Induction on  k. In the induction step  k → k + 1, consider using several copies of the graph you found for  k.

23.  −  Vertex degrees.

24.

Kn,n. To choose  n  so that  Kn,n  is not even  k-choosable, consider lists of  k-subsets of a  k 2-set.

25.  − Vizing’s theorem.

26.

All you need are the definitions, Proposition 5.2.2, and a standard argument from Chapter 1.2.

27.+ Try induction on  r. In the induction step, you would like to to delete one pair of vertices and only one colour from the other vertices’ lists.

What can you say about the lists if this is impossible? This information alone will enable you to find a colouring directly, without even looking at the graph again.

28.

Show that  χ00( G) 6 ch 0( G) + 2, and use this to deduce  χ00( G) 6

∆( G) + 3 from the list colouring conjecture.

29.  −  Do bipartite graphs have a kernel?

30.+ Call a set  S  of vertices in a directed graph  D  a  core  if  D  contains a directed  v– S  path for every vertex  v ∈ D − S. If, in addition,  D  contains no directed path between any two vertices of  S, call  S  a  strong core. Show first that every core contains a strong core. Next, define inductively a partition of  V ( D) into ‘levels’  L 0 , . . . , Ln  such that, for even  i,  Li  is a suitable strong core in  Di :=  D − ( L 0  ∪ . . . ∪ Li− 1), while for odd  i,  Li  consists of the vertices of  Di  that send an edge to  Li− 1.

Show that, if  D  has no directed odd cycle, the even levels together form a kernel of  D.

31.

Construct the orientation needed for Lemma 5.4.3 in steps: if, in the current orientation, there are still vertices  v  with  d+( v) > 3, reverse the directions of an edge at  v  and take care of the knock-on effect of this change. If you need to bound the average degree of a bipartite planar

graph, remember Euler’s formula.

32.  −  Start with a non-perfect graph.

33.  −  Do odd cycles or their complements satisfy ( ∗)?

34.

Exercise 12, Chapter 3.

290

Hints for all the exercises

35.

Look at the complement.

36.

Define the colour classes of a given induced subgraph  H ⊆ G  inductively, starting with the class of all minimal elements.

37.

(i) Can the vertices on an induced cycle contain each other as intervals?

(ii) Use the natural ordering of the reals.

38.

Compare  ω( H) with ∆( G) (where  H =  L( G)).

39.+ Which graphs are such that their line graphs contain no induced cycles of odd length > 5? To prove that the edges of such a graph  G  can be coloured with  ω( L( G)) colours, imitate the proof of Vizing’s theorem.

40.

Use  A  as a colour class.

41.+ (i) Induction.

(ii) Assume that  G  contains no induced  P  3. Suppose some  H  has a maximal complete subgraph  K  and a maximal set  A  of independent vertices disjoint from  K. For each vertex  v ∈ K, consider the set of neighbours of  v  in  A. How do these sets intersect? Is there a smallest one?

42.+ Start with a candidate for the set  O, i.e. a set of maximal complete subgraphs covering the vertex set of  G. If all the elements of  O  happen to have order  ω( G), how does the existence of  A  follow from the perfection of  G? If not, can you expand  G (maintaining perfection) so that they do and adapt the  A  for the expanded graph to  G?

43.+ Reduce the general case to the case when all but one of the  Gx  are trivial; then imitate the proof of Lemma 5.5.4.

44.

Apply the property of  H 1 to the graphs in  H 2, and vice versa.

Hints for Chapter 6

1.  −  Move the vertices, one by one, from  S  to  S. How does the value of f ( S, S) change each time?

2.

(i) Trick the algorithm into repeatedly using the middle edge in alter-

nating directions.

(ii) At any given time during the algorithm, consider for each vertex

v  the shortest  s– v  walk that qualifies as an initial segment of an augmenting path. Show for each  v  that the length of this  s– v  walk never decreases during the algorithm. Now consider an edge which is used

twice for an augmenting path, in the same direction. Show that the

second of these paths must have been longer than the first. Now derive

the desired bound.

3.+ For the edge version, define the capacity function so that a flow of maximum value gives rise to sufficiently many edge-disjoint paths. For the vertex version, split every vertex  x  into two adjacent vertices  x−, x+.

Define the edges of the new graph and their capacities in such a way

that positive flow through an edge  x−x+ corresponds to the use of  x by a path in  G.

Hints for Chapter 6

291

4.  − H-flows are nowhere zero, by definition.

5.  −  Use the definition and Proposition 6.1.1.

6.  −  Do subgraphs also count as minors?

7.  −  Try  k = 2 ,  3 , . . .  in turn. In searching for a  k-flow, tentatively fix the flow value through an edge and investigate which consequences this has

for the adjacent edges.

8.

To establish uniqueness, consider cuts of a special type.

9.

Express  G  as the union of cycles.

10.

Combine Z2 -flows on suitable subgraphs to a flow on  G.

11.+ Begin by sending a small amount of flow through every edge outside  T .

12.

View  G  as the union of suitably chosen cycles.

13.

Corollary 6.3.2 and Proposition 6.4.1.

14.  −  Duality.

15.

Take as  H  your favourite graph of large flow number. Can you decrease its flow number by adding edges?

16.

Euler.

17.+ Theorem 6.5.3.

18.  −  Search among small cubic graphs.

19.

Theorem 6.5.3.

20.

(i) Theorem 6.5.3.

(ii) Yes it can. Show, by considering a smallest counterexample, that

if every 3-connected cubic planar multigraph is 3-edge-colourable (and

hence has a 4-flow), then so is every bridgeless cubic planar multigraph.

21.+ For the ‘only if’ implication apply Proposition 6.1.1. Conversely, consider a circulation  f  on  G, with values in  {  0 , ± 1 , . . . , ±( k −  1)  }, that respects the given orientation (i.e. is positive or zero on the edge directions assigned by  D) and is zero on as few edges as possible. Then show that  f  is nowhere zero, as follows. If  f  is zero on  e =  st ∈ E  and D  directs  e  from  t  to  s, define a network  N = ( G, s, t, c) such that any flow in  N  of positive total value contradicts the choice of  f , but any cut in  N  of zero capacity contradicts the property assumed for  D.

22.  −  Convert the given multigraph into a graph with the same flow properties.

Hints for Chapter 7

1.  −  Straightforward from the definition.

2.  −  When constructing the graphs, start by fixing the colour classes.

3.

It is not difficult to determine an upper bound for ex( n, K 1 ,r). What remains to be proved is that this bound can be achieved for all  r  and  n.

4.

Proposition 1.7.2 (ii).

5.

Proposition 1.2.2 and Corollary 1.5.4.

292

Hints for all the exercises

6.+ What is the maximum number of edges in a graph of the structure given by Theorem 2.2.3 if it has no matching of size  k? What is the optimal distribution of vertices between  S  and the components of  G − S? Is there always a graph whose number of edges attains the corresponding

upper bound?

7.

Consider a vertex  x ∈ G  of maximum degree, and count the edges in  G − x.

8.

Choose  k  and  i  so that  n = ( r −  1) k +  i  with 0 6  i < r −  1. Treat the case of  i = 0 first, and then show for the general case that  tr− 1( n) =

¡ ¢

1  r− 2 ( n 2  − i 2) +  i .

2  r− 1

2

9.

The bounds given in the hint are the sizes of two particularly simple

Turán graphs—which ones?

10.+ How can you choose  v  so that the number of edges does not decrease?

Where in the graph can the operation be repeated, and what does the

situation look like when nothing new happens?

11.

Choose among the  m  vertices a set of  s  vertices that are still incident with as many edges as possible.

12.

For the first inequality, double the vertex set of an extremal graph for Ks,t  to obtain a bipartite graph with twice as many edges but still not containing a  Ks,t.

13.+ For the displayed inequality, count the pairs ( x, Y ) such that  x ∈ A  and Y ⊆ B, with  |Y | =  r  and  x  adjacent to all of  Y . For the bound on

¡ ¢

ex( n, K

s

r,r ), use the estimate ( s/t) t ≤

≤ st  and the fact that the

t

function  z 7→ zr  is convex.

14.

Assume that the upper density is larger than 1  −  1 . What does this r− 1

mean precisely, and what does the Erd˝

os-Stone theorem then imply?

15.

Corollary 1.5.4 and Proposition 1.2.2.

16.

Complete graphs.

17.  −  Average degree.

18.

Do 1 ( k −  1) n  edges force a subgraph of suitable minimum degree?

2

19.

Consider a longest path  P  in  G. Where do its endvertices have their neighbours? Can  G [  P ] contain a cycle on  V ( P )?

20.  −  Why would it be impractical to include, say, 1-element sets  X, Y  in the comparison?

21.  −  Apply the definition of an  ²-regular pair.

22.

Sparse graphs have few edges. How does that affect the average degree

condition in the definition of  ²-regularity?

Hints for Chapter 8

293

Hints for Chapter 8

1.

For the induction step, partition the vertex set of the given graph  G

into two sets  V 1 and  V 2 so that colourings of  G [  V 1 ] and  G [  V 2 ] can be combined to a colouring of  G.

2.

Imitate the start of the proof of Lemma 8.1.3.

3.  −  Does a large chromatic number force up the average degree? If in doubt, consult Chapter 5.

4.+ Try parallel paths in the grid as branch sets.

5.+ How can we best make a  T K 2 r  fit into a  Ks,s  when we want to keep  s small?

6.

Split the argument into the cases of  k = 0 and  k > 1.

7.

How are the two lemmas used in the proof of the theorem?

8.

Study the motivational chat preceding the definition of  f  in the proof.

9.+ Consider your favourite graphs with high average degree and low chromatic number. Which trees do they contain induced? Is there some

reason to expect that exactly these trees may always be found induced

in graphs of large average degree and small chromatic number?

10.  −  What does planarity have to do with minors?

11.  −  Consider a suitable supergraph.

12.  −  Average degree.

13.+ Show by induction on  |G|  that any 3-colouring of an induced cycle in G 6<  K 4 extends to all of  G.

14.+ Reduce the statement to critical  k-chromatic graphs and apply Vizing’s

theorem.

15.

(i) is easy. In the first part of (ii), distinguish between the cases that the graph is or is not separated by a  Kχ( G) − 1. Show the second part by induction on the chromatic number. In the induction step split the

vertex set of the graph into two subsets.

16.

Induction on the number of construction steps.

17.

Induction on  |G|.

18.

Note the previous exercise.

19.

Which of the graphs constructed as in Theorem 8.3.4 have the largest average degree?

20.

Which of the graphs constructed as in the hint have the largest average degree?

21.

Consider the subgraph of  G  induced by the neighbours of  x.

294

Hints for all the exercises

Hints for Chapter 9

1.  −  Can you colour the edges of  K 5 red and green without creating a red or a green triangle? Can you do the same for a  K 6?

2.

Induction on  c. In the induction step, unite two of the colour classes.

3.+ Choose a well-ordering of R, and compare it with the natural ordering.

Use the fact that countable unions of countable sets are countable.

4.+ The first and second question are easy. To prove the theorem of Erd˝

os

and Szekeres, use induction on  k  for fixed  `, and consider in the induction step the last elements of increasing subsequences of length  k.

Alternatively, apply Dilworth’s Theorem.

5.

Use the fact that  n > 4 points span a convex polygon if and only if every four of them do.

6.

Translate the given  k-partition of  {  1 ,  2 , . . . , n }  into a  k-colouring of the edges of  Kn.

7.

(i) is easy. For (ii) use the existence of  R(2 , k,  3).

8.

Begin by finding infinitely many sets whose pairwise intersections all

have the same size.

9.

The exercise offers more information than you need. Consult Chap-

ter 8.1 to see what is relevant.

10.

Consider an auxiliary graph whose vertices are coloured finite sub-

graphs of the given graph.

11.

Imitate the proof of Proposition 9.2.1.

12.

The lower bound is easy. Given a colouring for the upper bound, con-

sider a vertex and the neighbours joined to it by suitably coloured

edges.

13.  −  Given  H 1 and  H 2, construct a graph  H  for which the  G  of Theorem 9.3.1 satisfies ( ∗).

14.

Show inductively for  k = 0 , . . . , m  that  ω( Gk) =  ω( H).

15.

For the induction step, construct  G( H 1 , H 2) from the disjoint union of G( H 1 , H0 2) and  G( H0 1 , H 2) by joining some new vertices in a suitable way.

16.

Infinity lemma.

17.  −  How exactly does Proposition 9.4.1 fail if we delete  Kr  from the statement?

Hints for Chapter 10

1.

Consider the union of two colour classes.

2.

Will the proof of Proposition 10.1.2 go through if we assume  χ( G) >

|G|/k  instead of  α( G) 6  k? What do  k-connected graphs look like that satisfy the first condition but not the second?

3.

Examine an edge that gets added in one sequence but not in another.

Hints for Chapter 10

295

4.

Figure 10.1.1.

5.

Induction on  k  with  n  fixed; for the induction step consider  G.

6.  −  Recall the definition of a hamiltonian sequence.

7.  −  On which kind of vertices does the Chvátal condition come to bear?

To check the validity of the condition for  G, first find such a vertex.

8.

How does an arbitrary connected graph differ from the kind of graph

whose square contains a Hamilton cycle by Fleischner’s theorem? How could this difference obstruct the existence of a Hamilton cycle?

9.+ In the induction step consider a minimal cut.

10.

Condition ( ∗) in the proof of Fleischner’s theorem.

11.

Induction.

12.+ How can a Hamilton path  P ∈ H  be modified into another? In how many ways? What has this got to do with the degree in  G  of the last vertex of  P ?

Hints for Chapter 11

1.  −  Consider a fixed choice of  m  edges on  {  0 ,  1 , . . . , n }. What is the probability that  G ∈ G( n, p) has precisely this edge set?

2.

Consider the appropriate indicator random variables, as in the proof of

Lemma 11.1.5.

3.

Consider the appropriate indicator random variables.

4.

Erd˝

os.

5.

What would be the measure of the set  { G }  for a fixed  G?

6.

Consider the complementary properties.

7.  − P 2 ,  1.

8.

Apply Lemma 11.3.2.

9.

Induction on  |H|  with the aid of Exercise 6.

10.+ (i) Given a pair  U, U 0, calculate the probability that every other vertex is joined incorrectly to  U ∪ U 0. What, then, is the probability that this happens for  some  pair  U, U 0?

(ii) Enumerate the vertices of  G  and  G0  jointly, and construct an isomorphism  G → G0  inductively.

11.

Imitate the proof of Lemma 11.2.1.

12.

Imitate the proof of Proposition 11.3.1. To bound the probabilities involved, use the inequality 1  − x  6  e−x  as in the proof of Lemma

11.2.1.

13.+ (i) Calculate the expected number of isolated vertices, and apply Lemma 11.4.2 as in the proof of Theorem 11.4.3.

(ii) Linearity.

14.+ Chapter 8.2, the proof of Erd˝

os’s theorem, and a bit of Chebyshev.

296

Hints for all the exercises

15.

For the first problem modify an increasing property slightly, so that it ceases to be increasing but keeps its threshold function. For the second, look for an increasing property whose probability does not really depend on  p.

16.  −  Permutations of  V ( H).

17.  −  This is a result from the text in disguise.

18.  −  Balance.

19.

For  p/t →  0 apply Lemmas 11.1.4 and 11.1.5. For  p/t → ∞  apply Corollary 11.4.4.

20.

There are only finitely many trees of order  k.

21.+ Show first that no such threshold function  t =  t( n) can tend to zero as n → ∞. Then use Exercise 12.

22.+ Examine the various steps in the proof of Theorem 11.4.3, and note which changes will be needed. In the final steps of the proof, how are

the sums  AF  defined, and why is the sum of all the  AF  with  ||F || =  ∅

equal to  A 0? For  ||F || 6=  ∅, calculate a bound on  AF , and show that each  AF /µ 2 tends to zero as  n → ∞, as before.

Hints for Chapter 12

1.  −  Antisymmetry.

2.

Proposition 12.1.1.

3.

To prove Proposition 12.1.1, consider an infinite sequence in which every strictly decreasing subsequence is finite. How does the last element of a maximal decreasing subsequence compare with the elements

that come after it? For Corollary 12.1.2, start by proving that at least one element forms a good pair with infinitely many later elements.

4.

An obvious approach is to try to imitate the proof of Lemma 12.1.3

for 6 0; if it fails, what is the reason? Alternatively, you might try to modify the injective map produced by Lemma 12.1.3 into an order-preserving one, without losing the property of  a  6  f ( a) for all  a.

5.  −  This is an exercise in precision: ‘easy to see’ is not a proof . . .

6.

Start by finding two trees  T, T 0  with  |T | < |T 0|  but  T 6 6  T 0; then iterate.

7.

Does the original proof ever map the root of a tree to an ordinary vertex of another tree?

8.+ When we try to embed a graph  T G  in another graph  H, the branch vertices of the  T G  can be mapped only to certain vertices of  H. Enlarge G  to a similar graph  H  that does not contain  G  as a topological minor because these vertices of  H  are inconveniently positioned in  H. Then iterate this example to obtain an infinite antichain.

9.+ It is. One possible proof uses normal spanning trees with labels, and imitates the proof of Kruskal’s theorem.

10.

Why are there no cycles of tree-width 1?

Hints for Chapter 12

297

11.

For the forward implication, apply Corollary 1.5.2. For the converse, use induction on  k.

12.

To prove (T2), consider the edge  e  of Figure 12.3.1. Checking (T3) is easy.

13.

For the first question, recall Proposition 12.3.6. For the second, try to modify a tree-decomposition of  G  into one of the  T G  without increasing its width.

14.

Lemma 12.3.1 relates the separation properties of a graph  G  to those of its decomposition tree  T . This exercise illuminates this relationship from the dual viewpoint of connectedness: how are the connected

subgraphs of  G  related to those of  T ?

15.  −  This is just a reformulation of Theorem 12.3.9.

16.

Modify the proof given in the text that the  k × k  grid has tree-width at least  k −  1.

17.

Existence was shown in Theorem 12.3.9; the task is to show uniqueness.

18.+ Work out an explicit description of the sets  W 0t  similar to the definition of the  Wt, and compare the two.

19.

Induction.

20.

Induction.

21.

Use the previous exercise and a result from Chapter 8.3. And don’t

despair at a subgraph of  W !

22.+ Show that the parts are precisely the maximal irreducible induced subgraphs of  G.

23.+ Exercise 14.

24.

For the forward implication, interpret the subpaths of the decomposi-

tion path as intervals. Which subpath corresponds naturally to a given

vertex of  G?

25.

Follow the proof of Corollary 12.3.12.

26.+ They do. To prove it, show first that every connected graph  G  contains a path whose deletion decreases the path-width of  G.

Then apply

induction on a suitable set of trees, deleting a suitable path in the

induction step.

27.

Consider minimal sets such as  XP  in Proposition 12.5.1.

28.

To answer the first part, construct for each forbidden minor  X  a finite set of graphs whose exclusion as topological minors is equivalent to

forbidding  X  as a minor. For the second part recall Exercise 8.

29.

Find the required paths one by one.

30.+ One direction is just a weakening of Lemma 12.4.5. For the other, imitate the proof of Lemma 12.3.4.

31.+ Let  X  be an externally  `-connected set of  h  vertices in a graph  G, where h  and  `  are large. Consider a small separator  S  in  G: clearly, most of X  will lie in the same component of  G − S. Try to make these ‘large’

components, perhaps together with their separators  S, into the desired connected vertex sets.

298

Hints for all the exercises

32.

Consult Chapter 8.2 for substructures to be found in graphs of large

chromatic number.

33.

Derive the minor theorem first for connected graphs.

34.

K 5.

Index

Page numbers in italics refer to definitions; in the case of author names, they refer to theorems due to that author. The alphabetical order ignores letters that stand as variables; for example, ‘ k-colouring’ is listed under the letter c.

abstract

average degree,  5

dual,  88 –89

of bipartite planar graph, 289

graph, 3, 67, 76, 238

bounded, 210

acyclic,  12, 60

and chromatic number, 101, 106, 178,

adjacency matrix,  24

185

adjacent,  3

and connectivity, 11

Ahuja, R.K., 145

forcing minors, 169, 179, 184

algebraic

forcing topological minors, 61, 170–

colouring theory, 121

178

flow theory, 128–143

and girth, 237

graph theory, ix, 20–25, 28

and list colouring, 106

planarity criteria, 85–86

and minimum degree, 5–6

algorithmic graph theory, 145, 276–277,

and number of edges, 5

281–282

and Ramsey numbers, 210

almost,  238, 247–248

and regularity lemma, 154, 166

Alon, N.,  106, 121–122, 249

alternating

bad sequence,  252, 280

path,  29

balanced,  243

walk,  52

Behzad, M., 122

antichain,  40, 41, 42, 252

Berge, C., 117

Appel, K., 121

Berge graph,  117

arboricity,  61, 99, 118

between,  6, 68

arc,  68

Biggs, N.L., 28

Archdeacon, D., 281

bipartite graphs,  14 –15, 27, 91, 95

articulation point, see  cutvertex

edge colouring of, 103, 119

at,  2

flow number of cubic, 133–134

augmenting path

forced as subgraph, 152, 160

for matching,  29, 40, 285

list-chromatic index of, 109–110, 122

for network flow,  127, 144

matching in, 29–34

automorphism,  3

in Ramsey theory, 202–203

300

Index

Birkhoff, G.D., 121

and flow number, 139

block,  43

forcing minors, 181–185

graph,  44, 64

forcing short cycles, 101, 237

Böhme, T., 66

forcing subgraphs, 100–101, 178, 209

Bollobás, B., 28, 65, 66, 166,  170, 210,

forcing a triangle, 119, 209

227, 228, 240, 241, 249, 250

and girth, 101, 237

bond, see (minimal)  cut

as a global phenomenon, 101, 110

space, see  cut space

and maximum degree, 99

Bondy, J.A., 228

and minimum degree, 99, 100

boundary

and number of edges, 98

of a face,  72 –73

chromatic polynomial,  118, 146

bounded subset of R2,  70

Chvátal, V.,  194,  215,  216, 228

bramble,  258 –260, 281

circle on  S 2,  70

number,  260, 278

circuit, see  cycle

order of,  258

circulation,  124, 137, 146

branch

circumference,  7

set,  16

and connectivity, 64, 214

vertex,  18

and minimum degree, 8

bridge,  10, 36, 125, 135, 215

class 1 vs. class 2,  105

to bridge,  218

clique number,  110 –117, 202, 262

Brooks, R.L.,  99, 118

of random graph, 232

theorem,  99

threshold function, 247

list colouring version, 121

closed

Burr, S.A., 210

under addition, 128

under isomorphism, 238, 263

capacity,  126

wrt. minors, 119, 144, 263

function,  125

wrt. subgraphs, 119

Catlin, P.A., 187

wrt. supergraphs, 241

Cayley, A., 121, 248

walk,  9, 19

central vertex,  9, 283

cocycle space, see  cut space

certificate, 111, 274, 282

k-colourable,  95

chain,  13,  40, 41

colour class,  95

Chebyshev inequality, 243, 295

colour-critical, see  critically k-chromatic

Chen, G., 210

colouring,  95 –122

choice number,  105

algorithms, 98, 117

and average degree, 106

and flows, 136–139

of bipartite planar graphs, 119

number,  99, 118, 119

of planar graphs, 106

plane graphs, 96–97, 136–139

k-choosable,  105

in Ramsey theory,  191

chord,  7

total,  119

chordal,  111 –112, 120, 262, 279

3-colour theorem, see  three colour thm.

k-chromatic,  95

4-colour theorem, see  four colour thm.

chromatic index,  96, 103

5-colour theorem, see  five colour thm.

of bipartite graphs, 103

combinatorial

vs. list-chromatic index, 105, 108

isomorphism,  77, 78

and maximum degree, 103–105

set theory, 210

chromatic number,  95, 139

compactness argument, 191, 210

and  Kr-subgraphs, 100–101, 110–111

comparability graph, 111,  119

of almost all graphs, 240

complement

and average degree, 101, 106, 178,

of a bipartite graph, 111, 119

185

of a graph,  4

vs. choice number, 105–106

and perfection, 112, 290

and connectivity, 100

of a property,  263

and extremal graphs, 151

complete,  3

Index

301

bipartite,  14

by vertices,  30,  258

matching, see  1-factor

critical,  118

minor, 179–184, 275

critically  k-chromatic,  118, 293

multipartite,  14, 151

cross-edges,  21,  58

part of path-decomposition, 279

crosses in grid,  258

part of tree-decomposition, 262

crown,  208

r-partite,  14

cube

separator, 261, 279

d-dimensional,  26, 248

subgraph, 101, 110–111, 147–151,

of a graph,  G 3, 227

232, 247, 257

cubic graph,  5

topological minor, 61–62, 170–178,

connectivity of, 64

184, 186

1-factor in, 36, 41

complexity theory, 111, 274, 282

flow number of, 133–134, 135

component,  10

cut,  21

connected,  9

capacity of,  126

2-connected graphs, 43–45

-cycle duality, 136–138

3-connected graphs, 45–49, 79–80

-edge, see  bridge

k-connected,  10, 64

flow across, 125

externally,  264, 280

minimal, 22, 88

minimally connected, 12

in network,  126

minimally  k-connected,  65

space,  22 –24, 28, 85, 89

and vertex enumeration, 9, 13

cutvertex,  10, 43–44

connectedness,  9, 12, 297

cycle,  7 –9

connectivity,  10 –11, 43–66

-cut duality, 136–138

and average degree, 11

directed, 119

and circumference, 64

double cover conjecture, 141,  144

and edge-connectivity, 11

expected number, 234

external,  264, 280

Hamilton, 144,  213 –228

and girth, 237

induced, 7–8, 21, 47, 86, 111, 117,

and Hamilton cycles, 215

290

and linkability, 62, 65

length,  7

and minimum degree, 11

long, 8, 26, 64, 118

and plane duality, 91

in multigraphs, 25

and plane representation, 79–80

non-separating, 47, 86

Ramsey properties, 207–208

odd,  15, 99, 117, 290

of a random graph, 239

with orientation,  136 –138

k-constructible,  101 – 102, 118

short, 101, 179–180, 235, 237

contains,  3

space,  21, 23–24, 27–28, 47–49, 85–

contraction,  16 –18

86, 89, 92–93

and 3-connectedness, 45–46

threshold function, 247

and minors, 16–18

cyclomatic number,  21

in multigraphs, 25–26

and tree-width, 256

degeneracy, see  colouring number

convex

degree,  5

drawing, 82, 90, 92

sequence,  216

polygon, 209

deletion,  4

core,  289

∆-system,  209

cover

dense graphs,  148, 150

by antichains, 41

density

of a bramble,  258

edge density,  148

by chains, 40, 42

of pair of vertex sets,  153

by edges, 119

upper density, 166

by paths, 39–40

depth-first search tree,  13, 27

by trees, 60–61, 89

Deuber, W.,  197

302

Index

diameter,  8 –9, 248

forcing subgraphs, 147–167

and girth, 8

forcing minors/topological minors,

and radius, 9

169–180

Diestel, R., 186, 281

and regularity lemma, 154, 166

difference of graphs,  4

edge-disjoint spanning trees, 58–61

digon, see  double edge

edge-maximal,  4

digraph, see  directed graph

vs. extremal, 149, 182

Dilworth, R.P.,  40, 285, 294

without  M K 5, 183

Dirac, G.A.,  111, 186, 187,  214, 226

without  T K 4, 182

directed

without  T K 5,  T K 3 ,  3, 84

cycle, 119

without  T K 3 ,  3, 185

edge,  25

edge space,  20, 85

graph,  25, 108, 119

Edmonds, J., 42, 282

path,  39

embedding

direction,  124

of bipartite graphs,  202 –204

disjoint graphs,  3

in the plane,  76, 80–93

distance,  8

in  S 2, 69–70, 77

double

in surface, 74, 92, 274–276, 280, 281–

counting, 75,  92, 114–115, 234, 244

282

edge,  25

empty graph,  2

wheel,  208

end

drawing, 67,  76 –80

of edge,  2,  25

convex, 92

of path,  6

straight-line, 90

endpoints of arc,  68

dual

endvertex,  2,  25

abstract,  88 –89, 91

terminal vertex,  25

and connectivity, 91

equivalence

plane,  87, 91

of planar embeddings, 76–80,  79, 90

duality

of points in R2,  68

cycles and cuts, 23–24, 88–89, 136

in quasi-order,  277

flows and colourings, 136–139, 291

Erd˝

os, P., 101, 121,  151, 152, 163, 166,

of plane multigraphs, 87–89

167, 187,  197, 208, 209, 210,  215,

228,  232, 235– 237,  243, 249, 295

edge,  2

Erd˝

os-Sós conjecture,  152, 166, 167

crossing a partition,  21

Euler, L., 18– 19,  74

directed,  25

Euler

double,  25

characteristic, 276

of a multigraph,  25

formula,  74 –75, 89, 90, 289

plane,  70

tour,  19 –20, 291

X– Y  edge,  2

Eulerian graph,  19

edge-chromatic number, see  chromatic

even

index

degree, 19, 33

edge colouring,  96, 103–105, 191

graph,  133, 135, 145

and flow number, 135

event,  231

and matchings, 119

evolution of random graphs, 241, 249

`-edge-connected,  10

exceptional set,  153

edge-connectivity,  11, 55, 58

excluded minors, see  forbidden minors

edge contraction,  16

existence proof, probabilistic, 121, 229,

and 3-connectedness, 45

233, 235–237

vs. minors, 17

expanding a vertex,  113

in multigraph,  25

expectation,  233 –234, 242

edge cover, 119

exterior face, see  outer face

edge density, 5,  148

externally  k-connected,  264, 280

and average degree, 5

extremal

Index

303

bipartite graph, 165

high connectivity, 11

vs. edge-maximal, 149, 182

induced trees, 178

graph theory, 147, 151, 160, 166

large chromatic number, 101–103

graph,  149 –150

linkability, 62–63, 66, 171–174

without  M K 5, 183

long cycles, 8, 26, 118, 213–228

without  T K 4, 182

long paths, 8, 166

without  T K 5, 184

minor with large minimum degree,

without  T K 3 ,  3, 185

174, 179

short cycles, 179–180, 237

face,  70

subgraph, 13, 147–167

factor,  29

tree, 13, 178

1-factor, 29–38

triangle, 119, 209

1-factor theorem,  35, 42, 66

Ford, L.R. Jr.,  127, 145

2-factor, 33

forest,  12

k-factor,  29

partitions, 60–61

factor-critical,  36, 285

minor, 281

Fajtlowicz, S., 187

four colour problem,  120, 186

fan,  55

four colour theorem,  96, 141, 145, 181,

-version of Menger’s theorem,  55

183, 185, 215, 227

finite graph,  2

history, 120–121

first order sentence, 239

four-flow conjecture,  140 –141

first point on frontier,  68

Frank, A., 65, 145

five colour theorem,  96, 121, 141

Frobenius, F.G, 42

list version,  106, 121

from  . . .  to,  6

five-flow conjecture,  140, 141

frontier,  68

Fleischner, H.,  218, 295

Fulkerson, D.R., 122,  127, 145

flow, 123–146,  125 – 126

H-flow,  128 –133

Gallai, T.,  39, 42, 66, 167

k-flow,  131 –134, 140–143, 145

Gallai-Edmonds matching theorem, 36–

2-flow, 133

38, 42

3-flow, 133–134, 141

Galvin, F.,  109

4-flow, 134–135, 140–141

Gasparian, G.S., 122

6-flow theorem,  141 –143

genus

-colouring duality, 136–139, 291

and colouring, 121

conjectures, 140–141

of a surface, 276

group-valued, 128–133, 144

of a graph, 90,  280

integral,  126, 128

geometric dual, see  plane dual

network flow,  125 –128, 291

Gibbons, A., 145

number,  131 –134, 140, 144

Gilmore, P.C., 120

in plane graphs, 136–139

girth,  7

polynomial,  130, 146

and average degree, 237

total value of,  126

and chromatic number, 101, 121,

forbidden minors

235–237

and chromatic number, 181–185

and connectivity, 237

expressed by,  263, 274–277

and diameter, 8

minimal set of, 274, 280, 281

and minimum degree, 8, 179–180, 237

planar, 264

and minors, 179–180

and tree-width, 263–274

and planarity, 89

forcibly hamiltonian, see  hamiltonian

and topological minors, 178

sequence

Godsil, C., 28

forcing

Golumbic, M.C., 122

M Kr, 179–184, 186

good

T K 5, 184, 187

characterization, 274,  282

T Kr, 61, 170–178, 186

pair,  252

304

Index

sequence,  252

Heawood, P.J., 121, 145

Gorbunov, K.Yu., 281

Heesch, H., 121

Göring, F., 66

hereditary graph property,  263, 274–277

Graham, R.L., 210

algorithmic decidability, 276–277

graph,  2 –4, 25,  26

Higman, D.G.,  252, 280

invariant,  3

Hoffman, A.J., 120

minor theorem, 251, 274–277,  275

hypergraph,  25

partition,  60

plane,  70 –76, 87–89, 96–97, 106–108,

in (a graph),  7

136–139

incidence,  2

process,  250

encoding of planar embedding, see

property,  238

combinatorial isomorphism

simple,  26

map, 25–26

graphic sequence, see  degree sequence

matrix,  24

graph-theoretical isomorphism,  77 –78

incident,  2,  72

greedy algorithm,  98, 108, 117

increasing property,  241, 248

grid, 90, 184,  258

independence number,  110 –117

minor, 260, 264–274

and connectivity, 214–215

theorem,  264

and Hamilton cycles, 215

tree-width of, 260, 278, 281

and long cycles, 118

Grötzsch, H.,  97, 141, 145

and path cover, 39

group-valued flow, 128–133

of random graph, 232, 248

Grünwald, T., 66

independent

Guthrie, F., 120

edges,  3, 29–38

Gyárfás, A., 178, 185

events, 231

paths,  7, 55, 56–57, 283

Hadwiger, H., 181, 186, 187

vertices,  3, 39, 110, 232

conjecture, 169–170,  181 –183, 185,

indicator random variable,  234, 295

186–187

induced subgraph,  3, 111, 116–117, 290

Hajnal, A.,  197, 210

of almost all graphs, 238, 248

Hajós, G.,  102, 187

cycle, 7–8, 21, 47, 75, 86, 111, 117,

construction, 101–102

290

Haken, W., 121

of all imperfect graphs, 116–117, 120

Halin, R., 65–66, 227, 280–281

of all large connected graphs, 207

Hall, P.,  31, 42

in Ramsey theory, 196–206

Hamilton, W.R., 227

in random graph, 232, 249

Hamilton closure,  226

tree, 178

Hamilton cycle,  213 –228

infinite graphs, ix,  2, 28, 41, 166, 209,

in almost all graphs, 241

248, 280

and degree sequence, 216–218, 226

infinity lemma,  192, 210, 294

in  G 2, 218–226

initial vertex,  25

in  G 3, 227

inner face,  70

and the four colour theorem, 215

inner vertex,  6

and 4-flows, 144, 215

integral

and minimum degree, 214

flow, 126, 128

in planar graphs, 215

function,  126

power of, 226

random variable, 242

sufficient conditions, 213–218

interior

Hamilton path,  213, 218

of an arc,  68

hamiltonian

of a path, ˚

P , 6– 7

graph,  213

internally disjoint, see  independent

sequence,  216

intersection,  3

Harant, J., 66

graph,  279

head, see  terminal vertex

interval graph,  120, 279

Index

305

into,  255

linear algebra, 20–25, 47–49, 85–86, 116

intuition, 70, 231

linear programming, 145

invariant,  3

linked

irreducible graph,  279

by an arc,  68

isolated vertex,  5, 248

by a path,  6

isomorphic,  3

k-linked,  61 –63, 66

isomorphism,  3

vs.  k-connected, 62, 65

of plane graphs, 76–80

( k, `)-linked,  170

isthmus, see  bridge

set,  170

tree-decomposition,  261

Jaeger, F., 146

vertices,  6,  68

Janson, S., 249

list

Jensen, T.R., 120, 146, 281

-chromatic index,  105, 108–110, 121–

Johnson, D., 282

122

join,  2

-chromatic number, see  choice num-

Jordan, C.,  68,  70

ber

Jung, H.A.,  62, 186

colouring,  105 –110, 121–122

bipartite graphs, 108–110, 119

Brooks’s theorem, 121

Kahn, J., 122

conjecture,  108, 119, 122

Karoński, M., 249

k-list-colourable, see  k-choosable

Kempe, A.B., 121, 227

logarithms, 1

kernel

loop,  25

of incidence matrix, 24

Lovász, L., 42,  112,  115, 121, 122, 167

of directed graph,  108 – 109,  119

ÃLuczak, T., 249, 250

Kirchhoff’s law,  123, 124

Klein four-group, 135

MacLane, S.,  85, 92

Kleitman, D.J., 121

Mader, W.,  11,  56 – 57,  61, 65, 66,  178, knotless graph,  277

184, 186, 187

knot theory, 146

Magnanti, T.L., 145

Kohayakawa, Y., 167

Mani, P.,  62

Kollár, J., 167

map colouring, 95–97, 117, 120, 136

Komlós, J., 167,  170, 186, 210, 226

Markov chain, 250

König, D.,  30, 42, 52,  103, 119,  192,

Markov’s inequality,  233, 237, 242, 244

210

marriage theorem,  31, 33, 42, 285

duality theorem,  30, 39, 111

matchable,  36

infinity lemma,  192, 210, 294

matching,  29 –42

Königsberg bridges, 19

in bipartite graphs, 29–34, 111

Kostochka, A.V.,  179

and edge colouring, 119

Kruskal, J.A.,  253, 280, 296

in general graphs, 34–38

Kuratowski, C.,  80 – 84, 274

of vertex set,  29

Kuratowski-type characterization, 90,

Máté, A., 210

274–275, 281–282

matroid theory, 66, 93

max-flow min-cut theorem, 125,  127,

Larman, D.G.,  62

144, 145

Latin square,  119

maximal,  4

leaf,  12, 27

acyclic graph, 12

lean tree-decomposition,  261

planar graph,  80, 84, 90, 92, 183, 185

length

plane graph,  73, 80

of a cycle,  7

maximum degree,  5

of a path,  6, 8

bounded, 161, 194

of a walk,  9

and chromatic number, 99

line (edge),  2

and chromatic index, 103–105

graph,  4, 96, 185

and list-chromatic index, 110, 122

306

Index

and radius, 9, 26

first, see  Markov’s inequality

and Ramsey numbers, 194–196

second, 242–247

Menger, K., 42,  50 – 55, 64, 144, 288

monochromatic (in Ramsey theory)

k-mesh,  265

induced subgraph, 196–206

Milgram, A.N.,  39

(vertex) set,  191 –193

minimal,  4

subgraph,  191, 193–196

connected graph, 12

multigraph,  25 –26

k-connected graph,  65

list chromatic index of, 122

cut, 22, 88, 136

plane,  87

set of forbidden minors, 274, 280,

multiple edge,  25

281–282

Murty, U.S.R., 228

non-planar graph, 90

separating set,  63

Nash-Williams, C.St.J.A.,  58,  60, 66,

minimum degree,  5

280

and average degree, 5

neighbour,  3,  4

and choice number, 106

Nešetřil, J., 210, 211

and chromatic number, 99, 100

network,  125 –128

and circumference, 8

theory, 145

and connectivity, 11, 65–66

node (vertex),  2

forcing Hamilton cycle, 214, 226

normal tree,  13 –14, 27, 139, 144, 296

forcing long cycles, 8

nowhere

forcing long paths, 8, 166

dense, 61

forcing short cycles, 179–180, 237

zero,  128, 146

forcing trees, 13

null, see  empty

and girth, 178, 179–180, 237

and linkability, 171

minor, 16–19,  17

obstruction

K

to small tree-width, 258–260, 264–

3 ,  3, 92, 185

K 4, 182, 263

265, 280, 281

K 5, 183, 186

octahedron,  11, 15

K 5 and  K

odd

3 ,  3, 80–84

K 6, 183

component,  34

Kr, 180, 181

cycle,  15, 99, 117, 290

of all large 3- or 4-connected graphs,

degree, 5

208

on,  2

forbidden, 181–185,  263 –277, 279,

one-factor theorem,  35, 66

280, 281–282

Oporowski, B.,  208

forced, 174, 179–186

order

infinite, 280

of deletion/contraction, 17

of multigraph, 26

of a bramble,  258

Petersen graph, 140

of a graph,  2

and planarity, 80–84, 90

of a mesh or premesh,  265

relation, 18, 274

partial, 13, 18, 27, 40, 41, 120, 277

theorem, 251, 274–277,  275

quasi-,  251 –252, 277–278

for trees, 253–254

tree-,  13, 27

proof, 275–276

well-quasi-,  251 –253, 275, 277, 278,

vs. topological minor, 18–19, 80

280

and WQO, 251–277

orientable surface, 280

(see also  topological minor)

plane as, 137

Möbius

orientation,  25, 108, 145, 289

crown,  208

cycle with,  136 –137

ladder, 183

oriented graph,  25

Mohar, B., 92, 121, 281–282

Orlin, J.B., 145

moment

outer face,  70, 76–77

Index

307

outerplanar,  91

Plummer, M.D., 42

Oxley, J.G., 93,  208

point (vertex),  2

pointwise greater,  216

Palmer, E.M., 249

polygon,  68

parallel

polygonal arc,  68, 69

edges,  25

Pósa, L.,  197, 226

paths, 293

power of a graph,  218

parity, 5, 34, 37, 227

precision, 296

part of tree-decomposition,  255

premesh,  265

partially ordered set, 40, 41, 42

probabilistic method, 229, 235–238, 249

r-partite,  14

projective plane, 275, 281

partition,  1,  60, 191

Prömel, H.J., 117, 122

pasting,  111, 182, 183, 185, 261

property,  238

path,  6 –9

of almost all graphs, 238–241, 247–

a– b-path,  7, 55

248

A– B-path,  7, 50–55

hereditary,  263

H-path,  7, 44–45, 56–57, 64, 65, 66

increasing,  241

alternating,  29, 32

pseudo-random graph, 210

between given pairs of vertices, 61–

Pym, J.S., 66

63, 66, 170

cover,  39 –40, 285

quasi-ordering,  251 –252, 277–278

-decomposition,  279

directed,  39

disjoint paths, 39, 50–55

radius,  9

edge-disjoint, 55, 57, 58

and diameter, 9, 26

-hamiltonian sequence,  218

and maximum degree, 9, 26

independent paths,  7, 55, 56–57, 283

Rado, R., 210

induced, 207

Rado’s selection lemma, 210

length,  6

Ramsey, F.P.,  190 – 193

linkage, 61–63, 66, 170, 172

Ramsey

long, 8

graph,  197

-width,  279, 281

-minimal,  196

Pelikán, J., 185

numbers,  191,  193 –194, 209, 210, 232

perfect,  111 –117, 119–120, 122

Ramsey theory, 189–208

graph conjecture,  117

and connectivity, 207–208

graph theorem,  112, 115, 117, 122

induced, 196–206

matching, see  1-factor

infinite, 192, 208, 210

Petersen, J.,  33,  36

random graph, 179, 194, 229–250,  231

Petersen graph,  140 –141

evolution, 241

physics, 146

infinite, 248

piecewise linear, 67

process,  250

planar,  80 –89, 274

uniform model,  250

embedding,  76, 80–93

random variable, 233

planarity criteria

indicator r.v.,  234, 295

Kuratowski, 84

reducible configuration,  121

MacLane, 85

Reed, B.A., 281

Tutte, 86

refining a partition,  1, 155–159

Whitney, 89

region,  68 –70

plane

on  S 2,  70

dual,  87

regular,  5, 33, 226

duality, 87–89, 91, 136–139, 288

²-regular

graph,  70 –76,

pair,  153, 166

multigraph,  87 –89, 136–139

partition,  153

triangulation,  73, 75, 261

regularity

308

Index

graph,  161

subgraph,  3

inflated,  Rs,  194

trees, 13, 14

lemma, 148, 153–164,  154, 167, 210

edge disjoint, 58–60

Rényi, A.,  243, 249

number of, 248

Richardson, M., 119

sparse graphs, 147, 169–185, 194

rigid-circuit, see  chordal

Spencer, J.H., 210, 249

Ř´ıha, S., 228

Sperner’s lemma,  41

Robertson, N., 66, 121,  183, 186,  257,

square

264,  275, 281

of graph, 218

Rödl, V., 167,  194,  197, 211

Latin,  119

Rónyai, L., 167

stability number, see  independence

root,  13

number

rooted tree,  13, 253, 278

stable set,  3

Rothschild, B.L., 210

standard basis,  20

Royle, G.F., 28

star,  15, 166, 196

Ruciński, A., 249

induced, 207

star-shape, 287

Sanders, D.P., 121

Steger, A., 117, 122

Sárközy, G.N., 226

Steinitz, E., 92

saturated, see  edge-maximal

stereographic projection, 69

Schelp, R.H., 210

Stone, A.H.,  151, 160

Schoenflies, A.M.,  70

straight line segment,  68

Schrijver, A., 145

strong core,  289

Schur, I, 209

subcontraction, see  minor

Scott, A.D., 167,  178, 209

subdividing vertex,  18

second moment,  242 –247

subdivision,  18

self-minor conjecture,  280

subgraph,  3

separate

of all large  k-connected graphs, 207–

a graph,  10, 50, 55, 56

208

the plane,  68

forced by edge density, 147–164

separating set,  10

of high connectivity, 11

sequential colouring, see  greedy algo-

induced,  3

rithm

of large minimum degree, 5–6, 99,

series-parallel,  185

118

k-set,  1

sum

set system, see  hypergraph

of edge sets,  20

Seymour, P.D., 66, 92, 121,  141,  183,

of flows, 133

186, 187, 226,  257,  258,  264,  275,

supergraph,  3

280, 281

symmetric difference, 20, 29–30, 40, 53

shift-graph, 209

system of distinct representatives, 41

Simonovits, M., 166, 167, 210

Szabó, T., 167

simple

Szekeres, G., 208, 209

basis,  85, 92–93

Szemerédi, E.,  154,  170, 186,  194, 226

graph,  26

see also  regularity lemma

simplicial tree-decomposition,  261, 275,

279, 281

sink,  125

tail, see  initial vertex

six-flow theorem,  141

Tait, P.G., 121, 227–228

snark,  141

tangle, 281

planar, 141, 145, 215

Tarsi, M., 121

Sós, V., 152, 166, 167

terminal vertex,  25

source,  125

Thomas, R., 121,  183,  208, 210,  258,

spanned subgraph,  3

280

spanning

Thomason, A.G., 66,  170,  179, 186, 241

Index

309

Thomassen, C., 65, 92,  106, 121,  179,

and forbidden minors, 263–274

185, 187, 228, 281, 282

of grid, 260, 278, 281

three colour theorem,  97

of a minor, 257

three-flow conjecture,  141

of a subdivision, 278

threshold function,  241 –247, 250

obstructions to small, 258–260, 264–

Toft, B., 120, 146

265, 280, 281

topological isomorphism,  76, 78, 88

triangle,  3

topological minor,  17 –18

triangulated, see  chordal

K 3 ,  3, 92, 185

triangulation, see  plane triangulation

K 4, 182, 185, 263

trivial graph,  2

K 5, 92, 184

Trotter, W.T.,  194

K 5 and  K 3 ,  3, 75, 80–84

Turán, P.,  150

K 5

−, 185

theorem,  150, 195

Kr, 61, 170–178

graph,  149 –152, 166, 292

of all large 2-connected graphs, 207

Tutte, W.T.,  35,  46,  47,  58, 65, 66,  86, forced by average degree, 61, 170–178

92,  128,  131,  139, 145, 146,  215,

forced by chromatic number, 181

228

forced by girth, 178

flow conjectures,  140 – 141

induced, 178

Tutte polynomial, 146

as order relation, 18

Tychonov, A.N., 210

vs. ordinary minor, 18–19, 80

and planarity, 75, 80–84, 90

unbalanced subgraph, 247, 249

tree (induced), 178

uniformity lemma, see  regularity lemma

and WQO of general graphs, 278

union,  3

and WQO of trees, 253

unmatched,  29

torso,  279

upper density,  166

total chromatic number,  119

Urquhart, A., 121

total colouring,  119

conjecture,  119, 122

total value of a flow,  126

valency (degree),  5

touching sets,  258

value of a flow,  126

tournament,  227

variance,  242

transitive graph,  41

vertex,  2

travelling salesman problem, 227

-chromatic number,  95

tree,  12 –14

colouring,  95, 98–103

cover, 61

-connectivity,  10

as forced substructure, 13, 178, 185

cover,  30

normal,  13 –14, 27, 139, 144, 296

cut, see  separating set

-order,  13

of a plane graph,  70

threshold function for, 247

space,  20

well-quasi-ordering of trees, 253–254

-transitive,  41

tree-decomposition, 186,  255 –262, 278,

Vince, A., 249

280–281

Vizing, V.G.,  103, 121, 122, 289, 290,

induced on subgraphs, 256

293

induced on minors, 256

Voigt, M., 121

lean,  261

obstructions, 258–260, 264–265, 280,

Wagner, K.,  84, 93,  183, 184, 185, 186,

281

281

part of,  255

‘Wagner’s Conjecture’, 281

simplicial,  261, 275, 279, 281

Wagner graph,  183, 261–262, 279

width of,  257

walk,  9

tree-width,  257 –274

alternating,  52

and brambles, 258–260, 278, 281

closed,  9

duality theorem,  258 –260

length,  9

310

Index

well-ordering, 294

Whitney, H., 66,  80,  89

well-quasi-ordering,  251 –282

width of tree-decomposition,  257

Welsh, D.J.A., 146

Winkler, P., 249

wheel,  46

theorem, 46, 65

Zykov, A.A., 166

Symbol Index

The entries in this index are divided into two groups. Entries involving only mathematical symbols (i.e. no letters except variables) are listed on the first page, grouped loosely by logical function. The entry ‘[ ]’, for example, refers to the definition of induced subgraphs  H [  U ] on page 4

as well as to the definition of face boundaries  G [  f ] on page 72.

Entries involving fixed letters as constituent parts are listed on the

second page, in typographical groups ordered alphabetically by those

letters. Letters standing as variables are ignored in the ordering.

∅

2

h , i

19

=

3

/

15, 16, 24

'

3

C⊥, F⊥,  . . .

19

⊆

3

6

0 ,  1 ,  2,  . . .

1

251

4

( n)

16

k ,  . . .

232

E( v) , E0( w),  . . .

2

+

4, 19, 128

E( X, Y ) , E0( U, W ),  . . .

2

−

4, 70, 128

( e, x, y),  . . .

124

∈

2

→

→

→

E, F , C ,  . . .

124, 136, 138

r

70

←

←

←

∪

e, E , F ,  . . .

124

3

∩

f ( X, Y ) , g( U, W ),  . . .

124

3

∗

4

G∗, F ∗, →

e ∗,  . . .  88, 136, 140

G 2 , H 3,  . . .

216

b c

1

G, X, G,  . . .

4, 124, 258

d e

1

( S, S),  . . .

126

| |

2, 126

xy, x 1  . . . xk,  . . .

2, 7

k k

2, 153

xP, P x, xP y, xP yQz,  . . .  7

[ ]

4, 72

˚

P , ˚

xQ,  . . .

7, 68

[ ] k, [ ] <ω

1, 250

xT y,  . . .

13

312

Symbol Index

F2

19

col( G)

99

N

1

d( G)

5

Z n

1

d( v)

5

d+( v)

108

CG

34

d( x, y)

8

C( G)

20

d( X, Y )

153

C∗( G)

21

diam( G)

8

E( G)

19

ex( n, H)

149

G( n, p)

228

P

f ∗( v)

88

H

241

P

g( G)

7

i,j

236

V

i

1

( G)

19

init( e)

23

log ,  ln

1

Ck

7

pw( G)

259

E( G)

2

q( G)

34

E( X)

231

rad( G)

9

F ( G)

70

t

Forb

r− 1( n)

149

4( X )

257

ter( e)

23

G( H 1 , H 2)

196

tw( G)

255

Kn

3

v

K

e, vxy , vU

15, 16

n

14

1  ,...,nr

v∗( f )

88

Krs

14

L( G)

4

∆( G)

5

M X

15

N ( v) , N ( U )

4

α( G)

110

N +( v)

108

P

229

δ( G)

5

P k

6

ε( G)

5

P

κ( G)

10

G

118

R( H)

191

κG( H)

56

R( H

λ( G)

11

1 , H 2)

191

R( k, c, r)

191

λG( H)

56

R( r)

189

µ

240

R

π :  S 2 r  { (0 ,  0 ,  1)  } →  R2 69

s

161

Sn

69

σk : Z  →  Z k

131

T X

16

σ 2

240

T r− 1( n)

149

ϕ( G)

131

V ( G)

2

χ( G)

95

χ0( G)

96

ch( G)

105

χ00( G)

119

ch 0( G)

105

ω( G)

110

Reinhard Diestel received a PhD from the University of Cambridge, following research 1983–86 as a scholar of Trinity College under Béla Bollobás. He was a Fellow of St. John’s College, Cambridge, from 1986 to 1990. Research appointments and scholarships have taken him to Bielefeld (Germany), Oxford and the US. He became a professor in Chemnitz in 1994 and has held a chair at Hamburg since 1999.

Reinhard Diestel’s main area of research is graph theory, including infinite graph theory. He has published numerous papers and a research monograph, Graph Decompositions (Oxford 1990).

Document Outline

Reinhard Diestel Graph Theory

Preface 1st edition

Preface 2nd edition

Contents

1. The Basics

1.1   Graphs

1.2  The degree of a vertex

1.3  Paths and cycles

1.4  Connectivity

1.5  Trees and forests

1.6  Bipartite graphs

1.7  Contraction and minors

1.8  Euler tours

1.9  Some linear algebra

1.10  Other notions of graphs

Exercises

Notes

2.  Matching

2.1  Matching in bipartite graphs

2.2  Matching in general graphs

2.3  Path covers

Exercises

Notes

3.  Connectivity

3.1  2-Connected graphs and subgraphs

3.2  The structure of 3-connected graphs

3.3  Menger's theorem

3.4  Mader's theorem

3.5  Edge-disjoint spanning trees

3.6  Paths between given pairs of vertices

Exercises

Notes

4.  Planar Graphs

4.1  Topological prerequisites

4.2  Plane graphs

4.3  Drawings

4.4  Planar graphs: Kuratowski's theorem

4.5  Algebraic planarity criteria

4.6  Plane duality

Exercises

Notes

5.  Colouring

5.1  Colouring maps and planar graphs

5.2  Colouring vertices

5.3  Colouring edges

5.4  List colouring

5.5  Perfect graphs

Exercises

Notes

6.  Flows

6.1  Circulations

6.2  Flows  in networks

6.3  Group-valued flows

6.4  k-Flows for small k

6.5  Flow-colouring duality

6.6  Tutte's flow conjectures

Exercises

Notes

7.  Substructures in Dense Graphs

7.1  Subgraphs

7.2  Szemerédi's regularity lemma

7.3  Applying the regularity lemma

Exercises

Notes

8.  Substructures in Sparse Graphs

8.1  Topological minors

8.2  Minors

8.3  Hadwiger's conjecture

Exercises

Notes

9.  Ramsey Theory for Graphs

9.1  Ramsey's original theorems

9.2  Ramsey numbers

9.3  Induced Ramsey theorems

9.4  Ramsey properties and connectivity

Exercises

Notes

10. Hamilton Cycles

10.1  Simple sufficient conditions

10.2  Hamilton cycles and degree sequences

10.3  Hamilton cycles in the square of a graph

Exercises

Notes

11.  Random Graphs

11.1  The notion of a random graph

11.2  The probabilistic method

11.3 Properties of almost all graphs

11.4  Threshold functions and second moments

Exercises

Notes

12.  Minors, Trees, and WQO

12.1  Well-quasi-ordering

12.2  The graph minor theorem for trees

12.3  Tree-decompositions

12.4  Tree-width and forbidden minors

12.5  The graph minor theorem

Exercises

Notes

Hints for all the Exercises

Hints for Chapter 1

Hints for Chapter 2

Hints for Chapter 3

Hints for Chapter 4

Hints for Chapter 5

Hints for Chapter 6

Hints for Chapter 7

Hints for Chapter 8

Hints for Chapter 9

Hints for Chapter 10

Hints for Chapter 11

Hints for Chapter 12

Index

Symbol Index