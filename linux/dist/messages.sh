tag() {
  messages_tag=$(printf '%s' "${1%%[_.@-]*}" | tr 'A-Z' 'a-z')
  case $messages_tag in
    '' | ? | ?????????* | *[!a-z]* | posix) return 1 ;;
  esac
  printf '%s\n' "$messages_tag"
}

language() {
  messages_base=$(tag "${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}") || messages_base=
  messages_rest=${LANGUAGE:-}
  [ -n "$messages_rest" ] || messages_rest=$messages_base
  [ -n "$messages_base" ] || messages_rest=
  while [ -n "$messages_rest" ]; do
    messages_item=${messages_rest%%:*}
    case $messages_rest in
      *:*) messages_rest=${messages_rest#*:} ;;
      *) messages_rest= ;;
    esac
    case ${messages_item%%[.@]*} in
      C | POSIX) break ;;
    esac
    messages_name=$(tag "$messages_item") || continue
    if [ -f "$catalogs/$messages_name.ftl" ]; then
      echo "$messages_name"
      return
    fi
  done
  echo en
}

text() {
  messages_key=$1
  shift
  messages_args=
  for messages_pair in "$@"; do
    messages_clean=$(printf '%s' "$messages_pair" | LC_ALL=C tr -d '\000-\037\177' | LC_ALL=C awk '{ gsub(/\302[\200-\237]|\342\200[\252-\256]|\342\201[\246-\251]/, ""); printf "%s", $0 }')
    messages_args="$messages_args$messages_clean
"
  done
  LENTHM_TEXT_ARGS=$messages_args awk -v key="$messages_key" '
    BEGIN {
      count = split(ENVIRON["LENTHM_TEXT_ARGS"], pairs, "\n")
      for (i = 1; i <= count; i++) {
        at = index(pairs[i], "=")
        if (at > 1) value[substr(pairs[i], 1, at - 1)] = substr(pairs[i], at + 1)
      }
      prefix = key " = "
    }
    index($0, prefix) == 1 {
      rest = substr($0, length(prefix) + 1)
      out = ""
      while ((at = index(rest, "{ $")) > 0) {
        tail = substr(rest, at + 3)
        end = index(tail, " }")
        if (end == 0) break
        out = out substr(rest, 1, at - 1) value[substr(tail, 1, end - 1)]
        rest = substr(tail, end + 2)
      }
      print out rest
      found = 1
      exit
    }
    END { if (!found) print key }
  ' "$catalogs/$language.ftl" 2>/dev/null || printf '%s\n' "$messages_key"
}
