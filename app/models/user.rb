class User < ApplicationRecord
  # Include default devise modules. Others available are:
  # :confirmable, :lockable, :timeoutable, :trackable and :omniauthable
  devise :database_authenticatable, :registerable,
         :recoverable, :rememberable, :validatable

  has_many :adventures
  has_many :fights, dependent: :destroy

  belongs_to :current_adventure, class_name: 'Adventure', required: false
end
