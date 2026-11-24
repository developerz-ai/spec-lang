// Projected by hand from @cases of fn is_title, fn is_tag (src/todo/model.slg) and Text.size, Text.trim (docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.
#include <string>

#include "harness.hpp"
#include "todo/model/text.hpp"
#include "todo/model/types.hpp"
#include "todo/model/unicode.hpp"

using todo::model::is_tag;
using todo::model::is_title;

TEST(is_title_cases) {
  CHECK_EQ(is_title("milk"), true);
  CHECK_EQ(is_title(""), false);
  CHECK_EQ(is_title("   "), false);
  CHECK_EQ(is_title("a\tb"), false);
  CHECK_EQ(is_title("　"), false);
  CHECK_EQ(is_title("a\u2028b"), false);
  CHECK_EQ(is_title("a\u0085b"), false);
}

TEST(is_tag_cases) {
  CHECK_EQ(is_tag("home"), true);
  CHECK_EQ(is_tag("deep-work"), true);
  CHECK_EQ(is_tag("Home"), false);
  CHECK_EQ(is_tag(""), false);
  CHECK_EQ(is_tag("a b"), false);
  CHECK_EQ(is_tag("-x"), false);
  CHECK_EQ(is_tag("--all"), false);
}

TEST(trim_removes_exactly_the_25_white_space_code_points) {
  const char32_t white[] = {0x09,   0x0A,   0x0B,   0x0C,   0x0D,   0x20,   0x85,   0xA0,   0x1680,
                            0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008,
                            0x2009, 0x200A, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000};
  int count = 0;
  for (char32_t c = 0; c <= 0x10FFFF; ++c) {
    count += todo::model::unicode::is_white_space(c) ? 1 : 0;
  }
  CHECK_EQ(count, 25);
  for (const char32_t c : white) {
    std::string s;
    todo::model::unicode::append_utf8(s, c);
    CHECK(todo::model::unicode::is_blank(s));
  }
  CHECK(is_title("﻿"));
  CHECK(is_title("​"));
  CHECK(is_title("᠎"));
  CHECK(!is_title("   "));
}

TEST(title_size_counts_code_points) {
  std::string e200;
  std::string x200(200, 'x');
  std::string smile200;
  for (int i = 0; i < 200; ++i) {
    e200 += "é";
    smile200 += "\U0001F600";
  }
  CHECK(is_title(e200));
  CHECK(is_title(x200));
  CHECK(is_title(smile200));
  CHECK(!is_title(e200 + "é"));
  CHECK(!is_title(x200 + "x"));
  CHECK(!is_title(smile200 + "\U0001F600"));
}

TEST(title_refuses_controls_and_line_breaks_but_keeps_other_c1) {
  CHECK(!is_title("a\nb"));
  CHECK(
      !is_title("a\x7f"
                "b"));
  CHECK(!is_title(std::string("a\0b", 3)));
  CHECK(!is_title("a\u2029b"));
  CHECK(!is_title("é\u2028"));
  CHECK(is_title("a\u2027b\u0084c"));
  CHECK(is_title("  café  "));
}

TEST(title_must_be_unicode) {
  CHECK(!is_title("a\xff"));
  CHECK(!is_title("\xc0\xaf"));          // overlong
  CHECK(!is_title("\xed\xa0\x80"));      // a surrogate in UTF-8
  CHECK(!is_title("\xf4\x90\x80\x80"));  // above U+10FFFF
  CHECK(!is_title("\xe2\x82"));          // truncated
}

TEST(tag_bounds) {
  CHECK(is_tag(std::string(32, 'a')));
  CHECK(!is_tag(std::string(33, 'a')));
  CHECK(is_tag("0-9"));
  CHECK(!is_tag("café"));
  CHECK(!is_tag("a_b"));
}

TEST(validated_constructors_refuse) {
  CHECK(!todo::model::Title::make(" ").has_value());
  CHECK(!todo::model::Tag::make("X").has_value());
  CHECK(!todo::model::TaskNo::make(0).has_value());
  CHECK_EQ(todo::model::TaskNo::make(7)->value(), 7U);
}
