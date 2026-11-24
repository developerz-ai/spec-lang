#include "rt/suite.hpp"

#include <iostream>

namespace rt::test {

namespace {
std::string json(const std::string& text) {
  static const char* const hex = "0123456789abcdef";
  std::string out = "\"";
  for (const char c : text) {
    const auto byte = static_cast<unsigned char>(c);
    if (c == '"' || c == '\\') {
      out += '\\';
      out += c;
    } else if (byte < 0x20) {
      out += "\\u00";
      out += hex[byte >> 4];
      out += hex[byte & 0xF];
    } else {
      out += c;
    }
  }
  return out + "\"";
}
}  // namespace

Diag current_diag() {
  try {
    throw;
  } catch (const Stop& s) {
    return Diag{s.code, s.message};
  } catch (const Blocked& b) {
    return Diag{b.code, b.message};
  } catch (const CheckFailed& c) {
    return Diag{"F-TEST", c.message};
  } catch (const Failure& f) {
    return Diag{"F-TEST", "failed with " + show(f)};
  } catch (const std::exception& e) {
    return Diag{"E-RUNTIME", e.what()};
  }
}

void report(const std::string& check, const char* status, const std::optional<Diag>& diag) {
  std::string line = "{\"check\": " + json(check) + ", \"status\": \"" + status + "\"";
  if (diag) line += ", \"code\": " + json(diag->code) + ", \"diagnostic\": " + json(diag->message);
  std::cout << line << "}\n";
}

}  // namespace rt::test
