# CodeRabbit Review Profile - Assertive Mode
# This will make CodeRabbit provide more thorough and strict code reviews

# Set the review style to 'assertive' for more detailed feedback
reviews:
  profile: assertive
  instructions: "כתוב עבורי docstrings בעברית לקוד שלי."


# Additional settings can be added here to customize the review process
# For example:
# max_concurrent_reviews: 3
exclude_paths:
  - "**/test/**"
  - "**/vendor/**"
  - "**/target/**"
  - "**/node_modules/**"
  - "**/dist/**"

# You can also configure specific rules or override default behaviors
# rules:
#   - name: "function-complexity"
#     level: "warning"
#   - name: "file-length"
#     max_lines: 500
