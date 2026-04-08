class BooksController < ApplicationController
  def show
    t('.title')
  end
  def create
    t('.success')
  end
  def edit
    translate('.name')
  end
end
