use super::*;
#[test]
fn opaque_types() {
    reparse_test(
        "
            struct Bar {}
            trait Buz {}
            trait Baz {
                type Hi;
            }
            impl Buz for Bar {}
            impl Baz for Foo {
                type Hi = Foo;
            }
            opaque type Foo: Buz = Bar;
            ",
    );
}

#[test]
fn test_generic_opaque_types() {
    reparse_test(
        "
            struct Foo {}
            trait Bar<T> {}
            opaque type Baz<T>: Bar<T> = Foo;
            ",
    );
    reparse_test(
        "
            struct Foo<T> {}
            struct Unit {}
            trait Bar<T, U> {}
            opaque type Boz<U>: Bar<Unit, U> = Foo<U>;
            ",
    );
}

#[test]
fn test_opaque_type_as_type_value() {
    reparse_test(
        "
            struct Foo {}
            trait Bar {}
            trait Fuzz {
                type Assoc: Bar;
            }
            impl Bar for Foo {}
            impl Fuzz for Foo {
                type Assoc = Bax;
            }
            opaque type Bax: Bar = Foo;
            ",
    );
    reparse_test(
        "
            struct Foo {}
            trait Bar<T> {}
            trait Faz {
                type Assoc;
            }
            impl Faz for Foo {
                type Assoc = fn(Baz);
            }
            opaque type Baz: Bar<Foo> = Foo;
            ",
    );
}

// Generic opaque types can't currently be used as types (these fail to lower)
#[ignore]
#[test]
fn test_generic_opaque_type_as_value1() {
    reparse_test(
        "
            struct Foo {}
            trait Bar<T> {}
            trait Fizz {
                type Assoc: Bar<Foo>;
            }
            impl<T> Bar<T> for Foo {}
            impl Fizz for Foo {
                type Assoc = Baz<Foo>;
            }
            opaque type Baz<T>: Bar<T> = Foo;
            ",
    );
    reparse_test(
        "
            struct Foo {}
            trait Bar<T> {}
            trait Faz {
                type Assoc;
            }
            impl Faz for Foo {
                type Assoc = fn(Baz<Foo>);
            }
            opaque type Baz<T>: Bar<T> = Foo;
            ",
    );
    reparse_test(
        "
            struct Foo<T> {}
            struct Unit {}
            trait Bar<T, U> {}
            trait Fez {
                type Assoc;
            }
            impl Fez for Foo {
                type Assoc = fn(Biiiz<Unit>);
            }
            impl<T, U> Bar<T, U> for Foo {}
            opaque type Biiiz<U>: Bar<Unit, U> = Foo<U>;
            ",
    );
}