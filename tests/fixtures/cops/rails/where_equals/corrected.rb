User.where(name: 'Gabe')
User.where(name: nil)
User.where(name: ['john', 'jane'])
User.where(name: 'Gabe')
User.where(name: nil)
User.where(name: ['john', 'jane'])
User.where(name: 'Gabe')
Course.where(enrollments: { student_id: student.id })
scope :active, -> { where(active: true) }
