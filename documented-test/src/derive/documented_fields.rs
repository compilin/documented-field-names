use documented::{DocumentedFields, Error};

#[test]
fn it_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo {
        /// 1
        first: i32,
        /// 2
        second: i32,
    }

    assert_eq!(Foo::FIELD_DOCS.len(), 2);
    assert_eq!(Foo::get_field_docs("first"), Ok("1"));
    assert_eq!(Foo::get_field_docs("second"), Ok("2"));
    assert_eq!(
        Foo::get_field_docs("third"),
        Err(Error::NoSuchField("third".into()))
    );
}

#[test]
fn enum_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    enum Bar {
        /// 1
        First,
        /// 2
        Second,
    }

    assert_eq!(Bar::FIELD_DOCS.len(), 2);
    assert_eq!(Bar::get_field_docs("First"), Ok("1"));
    assert_eq!(Bar::get_field_docs("Second"), Ok("2"));
    assert_eq!(
        Bar::get_field_docs("Third"),
        Err(Error::NoSuchField("Third".into()))
    );
}

#[test]
fn union_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    union FooBar {
        /// 1
        first: i32,
        /// 2
        second: i32,
    }

    assert_eq!(FooBar::FIELD_DOCS.len(), 2);
    assert_eq!(FooBar::get_field_docs("first"), Ok("1"));
    assert_eq!(FooBar::get_field_docs("second"), Ok("2"));
}

#[test]
fn unnamed_fields() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo(
        /// 0
        i32,
        /// 1
        u32,
        /// 2
        i64,
    );

    assert_eq!(Foo::FIELD_DOCS.len(), 3);
    assert_eq!(Foo::FIELD_DOCS[0], "0");
    assert_eq!(Foo::FIELD_DOCS[1], "1");
    assert_eq!(Foo::FIELD_DOCS[2], "2");
}

#[test]
fn generic_type_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo<T> {
        /// foo
        foo: T,
    }

    assert_eq!(Foo::<u8>::get_field_docs("foo"), Ok("foo"));
}

#[test]
fn generic_type_with_bounds_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo<T: Copy> {
        /// foo
        foo: T,
    }

    assert_eq!(Foo::<u8>::get_field_docs("foo"), Ok("foo"));
}

#[test]
fn const_generic_type_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo<const LEN: usize> {
        /// foo
        foo: [u8; LEN],
    }

    assert_eq!(Foo::<69>::get_field_docs("foo"), Ok("foo"));
}

#[test]
fn lifetimed_type_works() {
    #[derive(DocumentedFields)]
    #[allow(dead_code)]
    struct Foo<'a> {
        /// foo
        foo: &'a u8,
    }

    assert_eq!(Foo::get_field_docs("foo"), Ok("foo"));
}

#[cfg(feature = "customise")]
mod test_customise {
    use documented::DocumentedFields;

    #[test]
    fn empty_customise_works() {
        #[derive(DocumentedFields)]
        #[documented_fields()]
        #[allow(dead_code)]
        struct Doge {
            /// Wow, much coin
            coin: usize,
        }

        assert_eq!(Doge::get_field_docs("coin"), Ok("Wow, much coin"));
    }

    #[test]
    fn multiple_attrs_works() {
        #[derive(DocumentedFields)]
        #[documented_fields()]
        #[documented_fields()]
        #[allow(dead_code)]
        struct Doge {
            /// Wow, much coin
            #[documented_fields()]
            #[documented_fields()]
            coin: usize,
        }

        assert_eq!(Doge::get_field_docs("coin"), Ok("Wow, much coin"));
    }

    #[test]
    fn container_customise_works() {
        #[derive(DocumentedFields)]
        #[documented_fields(trim = false)]
        #[allow(dead_code)]
        struct Doge {
            ///     Wow, much coin
            coin: usize,
            ///     Wow, much doge
            doge: bool,
        }

        assert_eq!(Doge::get_field_docs("coin"), Ok("     Wow, much coin"));
        assert_eq!(Doge::get_field_docs("doge"), Ok("     Wow, much doge"));
    }

    #[test]
    fn field_customise_works() {
        #[derive(DocumentedFields)]
        #[allow(dead_code)]
        struct Doge {
            ///     Wow, much coin
            #[documented_fields(trim = false)]
            coin: usize,
            ///     Wow, much doge
            doge: bool,
        }

        assert_eq!(Doge::get_field_docs("coin"), Ok("     Wow, much coin"));
        assert_eq!(Doge::get_field_docs("doge"), Ok("Wow, much doge"));
    }

    #[test]
    fn field_customise_override_works() {
        #[derive(DocumentedFields)]
        #[documented_fields(trim = false)]
        #[allow(dead_code)]
        struct Doge {
            ///     Wow, much coin
            #[documented_fields(trim = true)]
            coin: usize,
            ///     Wow, much doge
            doge: bool,
        }

        assert_eq!(Doge::get_field_docs("coin"), Ok("Wow, much coin"));
        assert_eq!(Doge::get_field_docs("doge"), Ok("     Wow, much doge"));
    }

    #[test]
    fn default_works() {
        #[derive(DocumentedFields)]
        #[documented_fields(default = "Woosh")]
        #[allow(dead_code)]
        enum Mission {
            /// Rumble
            Launch,
            Boost,
            // this is not very useful here, but for `*Opt` macros it is
            #[documented_fields(default = "Boom")]
            Touchdown,
        }

        assert_eq!(Mission::get_field_docs("Launch"), Ok("Rumble"));
        assert_eq!(Mission::get_field_docs("Boost"), Ok("Woosh"));
        assert_eq!(Mission::get_field_docs("Touchdown"), Ok("Boom"));
    }
}
